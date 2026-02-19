# ScreenerBot Build Scripts Analysis Report

## Overview

This report analyzes the current Electron build scripts compared to the old Tauri build scripts to identify missing automation features.

**Date:** December 25, 2025

---

## Executive Summary

The migration from Tauri to Electron left several critical automation features incomplete:

| Feature                   | OLD (Tauri)                   | CURRENT (Electron)            | Status       |
| ------------------------- | ----------------------------- | ----------------------------- | ------------ |
| Signal Handling (Ctrl+C)  | ✅ Comprehensive              | ❌ Missing                    | **MISSING**  |
| Emergency VM Shutdown     | ✅ All VMs stopped            | ❌ Partial                    | **PARTIAL**  |
| Build Lock Mechanism      | ✅ Stale lock cleanup         | ✅ Implemented                | ✓ OK         |
| VM Pre-flight Checks      | ✅ Auto-stop all VMs          | ❌ Missing in build-all.sh    | **MISSING**  |
| Clean Build Directory     | ✅ Wipes builds/ before start | ❌ Partial (--clean flag)     | **PARTIAL**  |
| Dry Run Mode              | ✅ In publish.sh              | ❌ Not in build scripts       | **MISSING**  |
| VM Auto-Start             | ✅ Comprehensive              | ✅ Implemented                | ✓ OK         |
| VM Auto-Pause/Stop        | ✅ After build                | ✅ Implemented                | ✓ OK         |
| VM Resource Optimization  | ✅ CPU/RAM tuning             | ❌ Missing                    | **MISSING**  |
| Build Artifacts Directory | `builds/<platform>/<arch>/`   | `builds/electron/<platform>/` | Changed (OK) |

---

## Detailed Analysis

### 1. build-all.sh - Master Build Script

#### OLD (Tauri) Features - 499 lines

```bash
# Signal Handling - COMPREHENSIVE
trap interrupted INT
trap 'emergency_shutdown_all_vms; echo ""; echo -e "${YELLOW}Terminated${NC}"; exit 143' TERM
trap 'emergency_shutdown_all_vms; exit 129' HUP

emergency_shutdown_all_vms() {
    echo -e "${YELLOW}[WARNING]${NC} Emergency: Stopping ALL running VMs..."
    prlctl list -a 2>/dev/null | grep -E 'running|paused|suspended' | awk '{print $1}' | while read vm_uuid; do
        prlctl stop "$vm_uuid" --kill 2>/dev/null || true
    done
}

# Pre-flight: Stop all VMs before starting
echo -e "${BLUE}Checking VM status...${NC}"
running_vms=$(prlctl list 2>/dev/null | awk '$2 == "running"' | wc -l)
if [ "$running_vms" -gt 0 ]; then
    echo -e "${YELLOW}Stopping $running_vms running VM(s) for clean build environment...${NC}"
    # ... stops all VMs
fi

# Clean build directory
log_info "Clearing builds directory for fresh artifacts..."
rm -rf "$OUTPUT_DIR/macos" "$OUTPUT_DIR/linux" "$OUTPUT_DIR/windows" 2>/dev/null || true

# Stale lock cleanup
if [ -f "$SCRIPT_DIR/.build.lock" ]; then
    lock_age=$(($(date +%s) - $(stat -f %m "$SCRIPT_DIR/.build.lock" 2>/dev/null || echo 0)))
    if [ $lock_age -gt 3600 ]; then
        echo -e "${YELLOW}Removing stale build lock (age: ${lock_age}s)${NC}"
        rm -f "$SCRIPT_DIR/.build.lock"
    fi
fi
```

#### CURRENT (Electron) Features - 374 lines

```bash
# Signal Handling - NONE
# No trap statements at all!

# Pre-flight - NONE
# No VM status check before starting

# Clean build - ONLY with --clean flag
if [ $CLEAN -eq 1 ]; then
    flags="$flags --clean"
fi

# Build lock - NOT IMPLEMENTED in build-all.sh
# Only in individual platform scripts
```

#### MISSING in build-all.sh:

1. **Signal handlers (INT, TERM, HUP)** - Critical for Ctrl+C cleanup
2. **Emergency VM shutdown function** - When interrupted, VMs are left running
3. **Pre-flight VM cleanup** - Doesn't stop running VMs before starting
4. **Stale build lock detection** - Not checking/cleaning old locks
5. **Automatic clean build** - Should wipe builds/electron/\* before starting (not just pass --clean)
6. **Dry run mode** - No --dry-run flag for testing

---

### 2. build-windows.sh - Windows VM Build

#### Current Implementation (425 lines) - GOOD

- ✅ Build lock mechanism with stale detection
- ✅ `kill_other_builds()` function
- ✅ `stop_other_vms()` function
- ✅ `ensure_vm_running()` function
- ✅ Cleanup trap for VM pause
- ✅ `--no-pause` flag support

#### MISSING:

1. **Dry run mode** - No --dry-run flag
2. **VM resource optimization** - Old script had:
   ```bash
   VM_CPUS=8
   VM_RAM=8192
   # Store/restore original settings
   ```

---

### 3. build-linux.sh - Linux Build

#### Current Implementation (412 lines) - GOOD

- ✅ Build lock mechanism with stale detection
- ✅ `kill_other_builds()` function
- ✅ `stop_other_vms()` function
- ✅ `ensure_vm_running()` function
- ✅ Cleanup trap for VM suspend

#### MISSING:

1. **Dry run mode** - No --dry-run flag
2. **VM resource optimization** - No CPU/RAM tuning

---

### 4. build-macos.sh - macOS Native Build

#### Current Implementation (345 lines) - OK

- ✅ Clean build with --clean flag
- ✅ Skip rust with --skip-rust flag

#### MISSING:

1. **Dry run mode** - No --dry-run flag
2. **Signal handling** - No cleanup on Ctrl+C

---

### 5. publish.sh - Publishing Script

#### Current Implementation (1699 lines) - COMPREHENSIVE

- ✅ --dry-run mode fully implemented
- ✅ Build triggering via build-all.sh
- ✅ SSH prerequisite checks
- ✅ Artifact validation

#### Connection to build scripts:

```bash
# Current: Calls build-all.sh with platform flags
if [ $SKIP_BUILD -eq 0 ]; then
    # Constructs: ./build-all.sh --macos --intel (etc)
    ./build-all.sh $build_flags
fi
```

---

## Automation Chain

### Expected Flow:

```
publish.sh
  └── Increment version
  └── build-all.sh [--macos] [--linux] [--windows]
        ├── Pre-flight: Stop all VMs
        ├── Clean builds/ directory
        ├── build-macos.sh
        │     └── Compile + Package
        ├── build-linux.sh
        │     ├── Acquire lock
        │     ├── Start Ubuntu VM
        │     ├── Compile + Package
        │     └── Suspend VM
        └── build-windows.sh
              ├── Acquire lock
              ├── Start Windows VM
              ├── Compile + Package
              └── Pause VM
  └── Upload artifacts
  └── Register with API
```

### Current Issues in Chain:

1. **build-all.sh doesn't clean before build** - Old artifacts may be mixed with new
2. **build-all.sh doesn't stop VMs first** - Resource contention if VMs running
3. **No signal handling** - Ctrl+C leaves VMs running and locks dangling

---

## Code Patterns to Port from Old Scripts

### 1. Signal Handling Block (Add to build-all.sh)

```bash
# ============================================================================
# Signal Handling (Ctrl+C)
# ============================================================================
emergency_shutdown_all_vms() {
    echo ""
    echo -e "${YELLOW}[$(date '+%H:%M:%S')] [WARNING]${NC} Emergency: Stopping ALL running VMs..."
    prlctl list -a 2>/dev/null | grep -E 'running|paused|suspended' | awk '{print $1}' | while read vm_uuid; do
        prlctl stop "$vm_uuid" --kill 2>/dev/null || true
    done
    echo -e "${GREEN}[$(date '+%H:%M:%S')] [INFO]${NC} All VMs stopped"
}

interrupted() {
    emergency_shutdown_all_vms
    echo ""
    echo -e "${YELLOW}[$(date '+%H:%M:%S')] [WARNING]${NC} Build interrupted by user (Ctrl+C)"
    exit 130
}

trap interrupted INT
trap 'emergency_shutdown_all_vms; echo ""; echo -e "${YELLOW}Terminated${NC}"; exit 143' TERM
trap 'emergency_shutdown_all_vms; exit 129' HUP
```

### 2. Pre-flight VM Cleanup (Add to build-all.sh)

```bash
# ============================================================================
# Pre-flight Checks and Automatic Cleanup
# ============================================================================

# Ensure all VMs are stopped before starting (clean slate)
echo -e "${BLUE}Checking VM status...${NC}"
running_vms=$(prlctl list 2>/dev/null | awk '$2 == "running"' | wc -l)
if [ "$running_vms" -gt 0 ]; then
    echo -e "${YELLOW}Stopping $running_vms running VM(s) for clean build environment...${NC}"
    prlctl list 2>/dev/null | awk '$2 == "running" {print $1}' | while read vm_uuid; do
        prlctl stop "$vm_uuid" --fast 2>/dev/null || true
    done
    sleep 2
    echo -e "${GREEN}VMs stopped${NC}"
fi
```

### 3. Clean Build Directory (Add to build-all.sh, before builds)

```bash
# Clear and recreate output directory for fresh build artifacts
log_info "Clearing builds directory for fresh artifacts..."
rm -rf "$OUTPUT_DIR/macos" "$OUTPUT_DIR/linux" "$OUTPUT_DIR/windows" 2>/dev/null || true
mkdir -p "$OUTPUT_DIR"
```

### 4. Stale Build Lock Check (Add to build-all.sh)

```bash
# Remove any stale build locks before starting
if [ -f "$SCRIPT_DIR/.build.lock" ]; then
    lock_age=$(($(date +%s) - $(stat -f %m "$SCRIPT_DIR/.build.lock" 2>/dev/null || echo 0)))
    if [ $lock_age -gt 3600 ]; then
        echo -e "${YELLOW}Removing stale build lock (age: ${lock_age}s)${NC}"
        rm -f "$SCRIPT_DIR/.build.lock"
    fi
fi
```

### 5. Dry Run Mode (Add to all build scripts)

```bash
# Add to options
DRY_RUN=0

# Add to argument parsing
--dry-run)
    DRY_RUN=1
    shift
    ;;

# Wrap actual build commands
if [ $DRY_RUN -eq 1 ]; then
    log_info "[DRY-RUN] Would build macOS..."
    log_info "[DRY-RUN] Would sync to VM..."
    log_info "[DRY-RUN] Would run cargo build..."
else
    # Actual build commands
fi
```

---

## Implementation Priority

### HIGH Priority (Safety & Reliability)

1. **Add signal handling to build-all.sh** - Prevents orphaned VMs and dangling locks
2. **Add pre-flight VM cleanup to build-all.sh** - Ensures clean build environment
3. **Add automatic clean build** - Remove old artifacts before new build

### MEDIUM Priority (Developer Experience)

4. **Add --dry-run to all build scripts** - Test automation without actual builds
5. **Add stale lock detection to build-all.sh** - Auto-recovery from crashed builds

### LOW Priority (Optimization)

6. **Add VM resource tuning** - Optimize CPU/RAM during builds
7. **Add build timing/metrics** - Track build performance

---

## Files to Modify

| File               | Changes Needed                                                                    |
| ------------------ | --------------------------------------------------------------------------------- |
| `build-all.sh`     | Add signal handling, pre-flight VM cleanup, auto-clean, stale lock check, dry-run |
| `build-macos.sh`   | Add dry-run mode                                                                  |
| `build-windows.sh` | Add dry-run mode                                                                  |
| `build-linux.sh`   | Add dry-run mode                                                                  |

---

## Estimated Effort

- **Signal handling + Pre-flight cleanup**: ~50 lines to add to build-all.sh
- **Dry-run mode**: ~20 lines per script (4 scripts = ~80 lines)
- **Testing**: Run each script with --dry-run, verify VM management

**Total: ~130 lines of code changes across 4 files**
