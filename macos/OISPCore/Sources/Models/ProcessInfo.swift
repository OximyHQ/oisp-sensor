// ProcessInfo.swift
// OISPCore
//
// Process information extracted from network flows

import Foundation
#if os(macOS)
import Darwin
import Darwin.bsm.libbsm
#endif

/// Information about a process making network requests
public struct OISPProcessInfo: Sendable {
    /// Process ID
    public let pid: pid_t

    /// Parent process ID
    public let ppid: pid_t

    /// Process name (short name, like "python3")
    public let comm: String

    /// Full path to executable
    public let exe: String

    /// User ID
    public let uid: uid_t

    /// Group ID
    public let gid: gid_t

    /// Timestamp when process info was captured
    public let capturedAt: Date

    public init(
        pid: pid_t,
        ppid: pid_t = 0,
        comm: String,
        exe: String,
        uid: uid_t = 0,
        gid: gid_t = 0,
        capturedAt: Date = Date()
    ) {
        self.pid = pid
        self.ppid = ppid
        self.comm = comm
        self.exe = exe
        self.uid = uid
        self.gid = gid
        self.capturedAt = capturedAt
    }

    /// Convert to RawEventMetadata for event emission
    public func toMetadata(fd: Int32? = nil) -> RawEventMetadata {
        RawEventMetadata(
            comm: comm,
            exe: exe,
            uid: UInt32(uid),
            fd: fd,
            ppid: UInt32(ppid)
        )
    }
}

// MARK: - Process Attribution

#if os(macOS)
import Darwin.POSIX

// MAXPATHLEN is typically 1024, PROC_PIDPATHINFO_MAXSIZE is 4*MAXPATHLEN
private let PROC_PATH_BUFFER_SIZE = 4096

/// Process attribution using libproc
public final class ProcessAttribution: @unchecked Sendable {
    public static let shared = ProcessAttribution()

    private init() {}

    /// Get process info for a given PID
    public func getProcessInfo(pid: pid_t) -> OISPProcessInfo? {
        // Get executable path
        var pathBuffer = [CChar](repeating: 0, count: PROC_PATH_BUFFER_SIZE)
        let pathLen = proc_pidpath(pid, &pathBuffer, UInt32(pathBuffer.count))

        guard pathLen > 0 else {
            return nil
        }

        let exePath = String(cString: pathBuffer)

        // Get process name from path
        let comm = (exePath as NSString).lastPathComponent

        // Get BSD info for PPID, UID, GID
        var bsdInfo = proc_bsdinfo()
        let infoSize = proc_pidinfo(
            pid,
            PROC_PIDTBSDINFO,
            0,
            &bsdInfo,
            Int32(MemoryLayout<proc_bsdinfo>.size)
        )

        let ppid: pid_t
        let uid: uid_t
        let gid: gid_t

        if infoSize > 0 {
            ppid = pid_t(bsdInfo.pbi_ppid)
            uid = bsdInfo.pbi_uid
            gid = bsdInfo.pbi_gid
        } else {
            ppid = 0
            uid = 0
            gid = 0
        }

        return OISPProcessInfo(
            pid: pid,
            ppid: ppid,
            comm: comm,
            exe: exePath,
            uid: uid,
            gid: gid
        )
    }

    /// Get PID from audit token data
    public func getPidFromAuditToken(_ tokenData: Data) -> pid_t? {
        guard tokenData.count >= MemoryLayout<audit_token_t>.size else {
            return nil
        }

        return tokenData.withUnsafeBytes { ptr -> pid_t? in
            guard let baseAddress = ptr.baseAddress else { return nil }
            let token = baseAddress.assumingMemoryBound(to: audit_token_t.self).pointee
            return audit_token_to_pid(token)
        }
    }

    /// Get process info from audit token
    public func getProcessInfo(auditToken: Data) -> OISPProcessInfo? {
        guard let pid = getPidFromAuditToken(auditToken) else {
            return nil
        }
        return getProcessInfo(pid: pid)
    }
}
#endif

// MARK: - CustomStringConvertible

extension OISPProcessInfo: CustomStringConvertible {
    public var description: String {
        "Process(\(pid): \(comm) at \(exe))"
    }
}

extension OISPProcessInfo: Codable {}
