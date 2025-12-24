# SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)
# Makefile for sslsniff - based on AgentSight's implementation
#
# Dependencies (git submodule, pinned to specific commits):
#   - bpftool: https://github.com/libbpf/bpftool @ 5b402ff
#   - libbpf (nested in bpftool/libbpf): @ 7a6e6b4
#
# Can be configured via environment variables:
#   LIBBPF_SRC   - Path to libbpf source (default: ../bpftool/libbpf/src)
#   BPFTOOL_SRC  - Path to bpftool source (default: ../bpftool/src)
#   VMLINUX_DIR  - Path to vmlinux headers (default: ../vmlinux)

OUTPUT := .output
CLANG ?= clang

# Default paths point to bpftool submodule (libbpf is nested inside)
LIBBPF_SRC ?= $(abspath ../bpftool/libbpf/src)
BPFTOOL_SRC ?= $(abspath ../bpftool/src)
VMLINUX_DIR ?= $(abspath ../vmlinux)

LIBBPF_OBJ := $(abspath $(OUTPUT)/libbpf.a)
BPFTOOL_OUTPUT ?= $(abspath $(OUTPUT)/bpftool)
BPFTOOL ?= $(BPFTOOL_OUTPUT)/bootstrap/bpftool

ARCH ?= $(shell uname -m | sed 's/x86_64/x86/' \
			 | sed 's/arm.*/arm/' \
			 | sed 's/aarch64/arm64/' \
			 | sed 's/ppc64le/powerpc/' \
			 | sed 's/mips.*/mips/' \
			 | sed 's/riscv64/riscv/' \
			 | sed 's/loongarch64/loongarch/')

VMLINUX := $(VMLINUX_DIR)/$(ARCH)/vmlinux.h

# Include paths - use libbpf's include from source
LIBBPF_INCLUDE := $(dir $(LIBBPF_SRC))include/uapi
INCLUDES := -I$(OUTPUT) -I$(LIBBPF_INCLUDE) -I$(dir $(VMLINUX))
CFLAGS := -g -Wall -O2
ALL_LDFLAGS := $(LDFLAGS) $(EXTRA_LDFLAGS)

CLANG_BPF_SYS_INCLUDES ?= $(shell $(CLANG) -v -E - </dev/null 2>&1 \
	| sed -n '/<...> search starts here:/,/End of search list./{ s| \(/.*\)|-idirafter \1|p }')

ifeq ($(V),1)
	Q =
	msg =
else
	Q = @
	msg = @printf '  %-8s %s%s\n' "$(1)" "$(patsubst $(abspath $(OUTPUT))/%,%,$(2))" "$(if $(3), $(3))";
	MAKEFLAGS += --no-print-directory
endif

.PHONY: all clean info

all: sslsniff

info:
	@echo "Build configuration:"
	@echo "  LIBBPF_SRC:  $(LIBBPF_SRC)"
	@echo "  BPFTOOL_SRC: $(BPFTOOL_SRC)"
	@echo "  VMLINUX:     $(VMLINUX)"
	@echo "  ARCH:        $(ARCH)"

clean:
	$(call msg,CLEAN)
	$(Q)rm -rf $(OUTPUT) sslsniff

$(OUTPUT) $(OUTPUT)/libbpf $(BPFTOOL_OUTPUT):
	$(call msg,MKDIR,$@)
	$(Q)mkdir -p $@

# Build libbpf
$(LIBBPF_OBJ): $(wildcard $(LIBBPF_SRC)/*.[ch] $(LIBBPF_SRC)/Makefile) | $(OUTPUT)/libbpf
	$(call msg,LIB,$@)
	$(Q)$(MAKE) -C $(LIBBPF_SRC) BUILD_STATIC_ONLY=1 \
		OBJDIR=$(dir $@)/libbpf DESTDIR=$(dir $@) \
		INCLUDEDIR= LIBDIR= UAPIDIR= \
		install

# Build bpftool
$(BPFTOOL): | $(BPFTOOL_OUTPUT)
	$(call msg,BPFTOOL,$@)
	$(Q)$(MAKE) ARCH= CROSS_COMPILE= OUTPUT=$(BPFTOOL_OUTPUT)/ -C $(BPFTOOL_SRC) bootstrap

# Build BPF code
$(OUTPUT)/sslsniff.bpf.o: sslsniff.bpf.c $(LIBBPF_OBJ) sslsniff.h $(VMLINUX) | $(OUTPUT) $(BPFTOOL)
	$(call msg,BPF,$@)
	$(Q)$(CLANG) -g -O2 -target bpf -D__TARGET_ARCH_$(ARCH) \
		$(INCLUDES) $(CLANG_BPF_SYS_INCLUDES) \
		-c $< -o $(OUTPUT)/sslsniff.tmp.bpf.o
	$(Q)$(BPFTOOL) gen object $@ $(OUTPUT)/sslsniff.tmp.bpf.o

# Generate BPF skeleton
$(OUTPUT)/sslsniff.skel.h: $(OUTPUT)/sslsniff.bpf.o | $(OUTPUT) $(BPFTOOL)
	$(call msg,GEN-SKEL,$@)
	$(Q)$(BPFTOOL) gen skeleton $< > $@

# Build userspace code
$(OUTPUT)/sslsniff.o: sslsniff.c $(OUTPUT)/sslsniff.skel.h sslsniff.h | $(OUTPUT)
	$(call msg,CC,$@)
	$(Q)$(CC) $(CFLAGS) $(INCLUDES) -c $< -o $@

# Link sslsniff binary
sslsniff: $(OUTPUT)/sslsniff.o $(LIBBPF_OBJ) | $(OUTPUT)
	$(call msg,BINARY,$@)
	$(Q)$(CC) $(CFLAGS) $^ $(ALL_LDFLAGS) -lelf -lz -o $@

.DELETE_ON_ERROR:
.SECONDARY:
