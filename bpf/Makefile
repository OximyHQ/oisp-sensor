# OISP Sensor eBPF Programs Makefile

CLANG ?= clang
LLVM_STRIP ?= llvm-strip
ARCH := $(shell uname -m | sed 's/x86_64/x86/' | sed 's/aarch64/arm64/')
VMLINUX := vmlinux/vmlinux.h

BPF_CFLAGS := -g -O2 -target bpf -D__TARGET_ARCH_$(ARCH) \
	-I. -I./vmlinux \
	-Wall -Wno-unused-value -Wno-pointer-sign \
	-Wno-compare-distinct-pointer-types

PROGS := ssl_monitor.bpf.o process_monitor.bpf.o

.PHONY: all clean vmlinux

all: $(PROGS)

vmlinux:
	@mkdir -p vmlinux
	@if [ ! -f $(VMLINUX) ]; then \
		echo "Generating vmlinux.h..."; \
		bpftool btf dump file /sys/kernel/btf/vmlinux format c > $(VMLINUX); \
	fi

%.bpf.o: %.bpf.c $(VMLINUX) | vmlinux
	$(CLANG) $(BPF_CFLAGS) -c $< -o $@
	$(LLVM_STRIP) -g $@

clean:
	rm -f *.bpf.o
	rm -rf vmlinux/

