// SPDX-License-Identifier: GPL-2.0 OR Apache-2.0
//
// OISP Sensor - SSL/TLS Monitor eBPF Program
//
// Attaches uprobes to SSL_read/SSL_write to capture plaintext data
// at the TLS boundary.

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

// Maximum data size per event
#define MAX_DATA_SIZE 16384

// Event types
#define SSL_READ  0
#define SSL_WRITE 1

// SSL data event structure
struct ssl_data_event {
    __u64 timestamp_ns;
    __u32 pid;
    __u32 tid;
    __u32 uid;
    __u32 data_len;
    __u8 type;  // SSL_READ or SSL_WRITE
    __u8 data[MAX_DATA_SIZE];
    char comm[16];
};

// Ring buffer for events
struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 256 * 1024);
} events SEC(".maps");

// Map to track SSL_write entry args
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10240);
    __type(key, __u64);  // pid_tgid
    __type(value, void*); // buffer pointer
} ssl_write_args SEC(".maps");

// Map to track SSL_read entry args
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10240);
    __type(key, __u64);
    __type(value, void*);
} ssl_read_args SEC(".maps");

// Helper to emit event
static __always_inline int emit_ssl_event(void *ctx, void *buf, int len, __u8 type) {
    if (len <= 0 || len > MAX_DATA_SIZE) {
        return 0;
    }
    
    struct ssl_data_event *event;
    event = bpf_ringbuf_reserve(&events, sizeof(*event), 0);
    if (!event) {
        return 0;
    }
    
    __u64 pid_tgid = bpf_get_current_pid_tgid();
    __u64 uid_gid = bpf_get_current_uid_gid();
    
    event->timestamp_ns = bpf_ktime_get_ns();
    event->pid = pid_tgid >> 32;
    event->tid = (__u32)pid_tgid;
    event->uid = (__u32)uid_gid;
    event->type = type;
    event->data_len = len;
    
    bpf_get_current_comm(&event->comm, sizeof(event->comm));
    
    // Read data from user buffer
    if (bpf_probe_read_user(event->data, len & (MAX_DATA_SIZE - 1), buf) != 0) {
        bpf_ringbuf_discard(event, 0);
        return 0;
    }
    
    bpf_ringbuf_submit(event, 0);
    return 0;
}

// SSL_write entry - save buffer pointer
SEC("uprobe/SSL_write")
int BPF_UPROBE(ssl_write_entry, void *ssl, const void *buf, int num) {
    __u64 pid_tgid = bpf_get_current_pid_tgid();
    bpf_map_update_elem(&ssl_write_args, &pid_tgid, &buf, BPF_ANY);
    return 0;
}

// SSL_write return - read the data that was written
SEC("uretprobe/SSL_write")
int BPF_URETPROBE(ssl_write_return, int ret) {
    __u64 pid_tgid = bpf_get_current_pid_tgid();
    
    void **buf_ptr = bpf_map_lookup_elem(&ssl_write_args, &pid_tgid);
    if (!buf_ptr) {
        return 0;
    }
    
    void *buf = *buf_ptr;
    bpf_map_delete_elem(&ssl_write_args, &pid_tgid);
    
    if (ret > 0) {
        emit_ssl_event(ctx, buf, ret, SSL_WRITE);
    }
    
    return 0;
}

// SSL_read entry - save buffer pointer
SEC("uprobe/SSL_read")
int BPF_UPROBE(ssl_read_entry, void *ssl, void *buf, int num) {
    __u64 pid_tgid = bpf_get_current_pid_tgid();
    bpf_map_update_elem(&ssl_read_args, &pid_tgid, &buf, BPF_ANY);
    return 0;
}

// SSL_read return - read the data that was received
SEC("uretprobe/SSL_read")
int BPF_URETPROBE(ssl_read_return, int ret) {
    __u64 pid_tgid = bpf_get_current_pid_tgid();
    
    void **buf_ptr = bpf_map_lookup_elem(&ssl_read_args, &pid_tgid);
    if (!buf_ptr) {
        return 0;
    }
    
    void *buf = *buf_ptr;
    bpf_map_delete_elem(&ssl_read_args, &pid_tgid);
    
    if (ret > 0) {
        emit_ssl_event(ctx, buf, ret, SSL_READ);
    }
    
    return 0;
}

char LICENSE[] SEC("license") = "Dual BSD/GPL";

