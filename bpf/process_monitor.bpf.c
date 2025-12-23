// SPDX-License-Identifier: GPL-2.0 OR Apache-2.0
//
// OISP Sensor - Process Monitor eBPF Program
//
// Monitors process execution, fork, and exit events.

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

#define TASK_COMM_LEN 16
#define MAX_ARGS_LEN 256
#define MAX_PATH_LEN 256

// Event types
#define PROC_EXEC 0
#define PROC_EXIT 1
#define PROC_FORK 2

// Process event structure
struct process_event {
    __u64 timestamp_ns;
    __u32 pid;
    __u32 ppid;
    __u32 uid;
    __u32 gid;
    __u8 type;
    __s32 exit_code;
    char comm[TASK_COMM_LEN];
    char exe[MAX_PATH_LEN];
    char args[MAX_ARGS_LEN];
};

// Ring buffer for events
struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 256 * 1024);
} events SEC(".maps");

// Trace sched_process_exec
SEC("tracepoint/sched/sched_process_exec")
int trace_exec(struct trace_event_raw_sched_process_exec *ctx) {
    struct process_event *event;
    event = bpf_ringbuf_reserve(&events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }
    
    __u64 pid_tgid = bpf_get_current_pid_tgid();
    __u64 uid_gid = bpf_get_current_uid_gid();
    
    struct task_struct *task = (struct task_struct *)bpf_get_current_task();
    
    event->timestamp_ns = bpf_ktime_get_ns();
    event->pid = pid_tgid >> 32;
    event->ppid = BPF_CORE_READ(task, real_parent, tgid);
    event->uid = (__u32)uid_gid;
    event->gid = uid_gid >> 32;
    event->type = PROC_EXEC;
    event->exit_code = 0;
    
    bpf_get_current_comm(&event->comm, sizeof(event->comm));
    
    // Read filename from tracepoint
    bpf_probe_read_str(event->exe, sizeof(event->exe), (void *)ctx->filename);
    
    bpf_ringbuf_submit(event, 0);
    return 0;
}

// Trace sched_process_exit
SEC("tracepoint/sched/sched_process_exit")
int trace_exit(struct trace_event_raw_sched_process_template *ctx) {
    struct process_event *event;
    event = bpf_ringbuf_reserve(&events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }
    
    __u64 pid_tgid = bpf_get_current_pid_tgid();
    __u64 uid_gid = bpf_get_current_uid_gid();
    
    struct task_struct *task = (struct task_struct *)bpf_get_current_task();
    
    event->timestamp_ns = bpf_ktime_get_ns();
    event->pid = pid_tgid >> 32;
    event->ppid = BPF_CORE_READ(task, real_parent, tgid);
    event->uid = (__u32)uid_gid;
    event->gid = uid_gid >> 32;
    event->type = PROC_EXIT;
    event->exit_code = BPF_CORE_READ(task, exit_code);
    
    bpf_get_current_comm(&event->comm, sizeof(event->comm));
    
    bpf_ringbuf_submit(event, 0);
    return 0;
}

// Trace sched_process_fork
SEC("tracepoint/sched/sched_process_fork")
int trace_fork(struct trace_event_raw_sched_process_fork *ctx) {
    struct process_event *event;
    event = bpf_ringbuf_reserve(&events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }
    
    event->timestamp_ns = bpf_ktime_get_ns();
    event->pid = ctx->parent_pid;
    event->ppid = ctx->parent_pid;
    event->type = PROC_FORK;
    event->exit_code = ctx->child_pid; // Store child PID in exit_code field
    
    bpf_probe_read_str(event->comm, sizeof(event->comm), ctx->parent_comm);
    
    bpf_ringbuf_submit(event, 0);
    return 0;
}

char LICENSE[] SEC("license") = "Dual BSD/GPL";

