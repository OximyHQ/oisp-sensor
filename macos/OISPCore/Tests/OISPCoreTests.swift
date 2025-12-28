// OISPCoreTests.swift
// OISPCore Tests

import XCTest
@testable import OISPCore

final class OISPCoreTests: XCTestCase {

    // MARK: - RawCaptureEvent Tests

    func testRawCaptureEventCreation() throws {
        let metadata = RawEventMetadata(
            comm: "python3",
            exe: "/usr/bin/python3",
            uid: 501,
            fd: 5,
            ppid: 1234
        )

        let event = RawCaptureEvent(
            kind: .sslWrite,
            pid: 5678,
            data: "Hello, World!".data(using: .utf8)!,
            metadata: metadata,
            remoteHost: "api.openai.com",
            remotePort: 443
        )

        XCTAssertEqual(event.kind, .sslWrite)
        XCTAssertEqual(event.pid, 5678)
        XCTAssertEqual(event.metadata.comm, "python3")
        XCTAssertEqual(event.remoteHost, "api.openai.com")
        XCTAssertEqual(event.remotePort, 443)
    }

    func testRawCaptureEventJSONSerialization() throws {
        let metadata = RawEventMetadata(
            comm: "curl",
            exe: "/usr/bin/curl",
            uid: 501
        )

        let event = RawCaptureEvent(
            kind: .sslRead,
            pid: 1234,
            data: "HTTP/1.1 200 OK".data(using: .utf8)!,
            metadata: metadata
        )

        let jsonData = try event.toJSON()
        let jsonString = String(data: jsonData, encoding: .utf8)!

        XCTAssertTrue(jsonString.contains("\"kind\":\"SslRead\""))
        XCTAssertTrue(jsonString.contains("\"pid\":1234"))
        XCTAssertTrue(jsonString.contains("\"comm\":\"curl\""))
    }

    // MARK: - AIEndpointFilter Tests

    func testAIEndpointFilterExactMatch() {
        let filter = AIEndpointFilter()

        XCTAssertTrue(filter.shouldIntercept(host: "api.openai.com", port: 443))
        XCTAssertTrue(filter.shouldIntercept(host: "api.anthropic.com", port: 443))
        XCTAssertTrue(filter.shouldIntercept(host: "api.mistral.ai", port: 443))
    }

    func testAIEndpointFilterNonMatch() {
        let filter = AIEndpointFilter()

        XCTAssertFalse(filter.shouldIntercept(host: "google.com", port: 443))
        XCTAssertFalse(filter.shouldIntercept(host: "api.stripe.com", port: 443))
        XCTAssertFalse(filter.shouldIntercept(host: "github.com", port: 443))
    }

    func testAIEndpointFilterPortFiltering() {
        let filter = AIEndpointFilter()

        // Should only intercept HTTPS ports
        XCTAssertTrue(filter.shouldIntercept(host: "api.openai.com", port: 443))
        XCTAssertFalse(filter.shouldIntercept(host: "api.openai.com", port: 80))
    }

    func testAIEndpointFilterSuffixMatch() {
        let filter = AIEndpointFilter()

        // Azure OpenAI uses suffix matching
        XCTAssertTrue(filter.shouldIntercept(host: "myresource.openai.azure.com", port: 443))
        XCTAssertTrue(filter.shouldIntercept(host: "another.openai.azure.com", port: 443))
    }

    func testProviderDetection() {
        let filter = AIEndpointFilter()

        XCTAssertEqual(filter.detectProvider(host: "api.openai.com"), .openai)
        XCTAssertEqual(filter.detectProvider(host: "api.anthropic.com"), .anthropic)
        XCTAssertEqual(filter.detectProvider(host: "api.mistral.ai"), .mistral)
        XCTAssertEqual(filter.detectProvider(host: "myresource.openai.azure.com"), .azure)
        XCTAssertEqual(filter.detectProvider(host: "localhost"), .local)
        XCTAssertEqual(filter.detectProvider(host: "unknown.example.com"), .unknown)
    }

    // MARK: - ProcessInfo Tests

    func testProcessInfoCreation() {
        let processInfo = OISPProcessInfo(
            pid: 1234,
            ppid: 1,
            comm: "python3",
            exe: "/usr/bin/python3",
            uid: 501,
            gid: 20
        )

        XCTAssertEqual(processInfo.pid, 1234)
        XCTAssertEqual(processInfo.ppid, 1)
        XCTAssertEqual(processInfo.comm, "python3")
        XCTAssertEqual(processInfo.exe, "/usr/bin/python3")
        XCTAssertEqual(processInfo.uid, 501)
    }

    func testProcessInfoToMetadata() {
        let processInfo = OISPProcessInfo(
            pid: 1234,
            ppid: 1,
            comm: "node",
            exe: "/usr/local/bin/node",
            uid: 501,
            gid: 20
        )

        let metadata = processInfo.toMetadata(fd: 10)

        XCTAssertEqual(metadata.comm, "node")
        XCTAssertEqual(metadata.exe, "/usr/local/bin/node")
        XCTAssertEqual(metadata.uid, 501)
        XCTAssertEqual(metadata.fd, 10)
        XCTAssertEqual(metadata.ppid, 1)
    }

    #if os(macOS)
    func testProcessAttributionCurrentProcess() {
        let attribution = ProcessAttribution.shared
        let currentPid = getpid()

        let info = attribution.getProcessInfo(pid: currentPid)

        XCTAssertNotNil(info)
        XCTAssertEqual(info?.pid, currentPid)
        XCTAssertFalse(info?.exe.isEmpty ?? true)
        XCTAssertFalse(info?.comm.isEmpty ?? true)
    }
    #endif
}
