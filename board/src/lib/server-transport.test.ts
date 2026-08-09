import { describe, expect, test } from "bun:test";

import {
	classifyServerTransport,
	inferServerInstallKind,
	isRemoteHttpTransport,
} from "./server-transport";

describe("classifyServerTransport", () => {
	test("recognizes stdio and process aliases", () => {
		expect(classifyServerTransport("stdio")).toBe("stdio");
		expect(classifyServerTransport("local_process")).toBe("stdio");
	});

	test("keeps sse distinct from streamable http", () => {
		expect(classifyServerTransport("sse")).toBe("sse");
		expect(classifyServerTransport("streamable_http")).toBe("streamable_http");
		expect(classifyServerTransport("stream")).toBe("streamable_http");
	});

	test("treats generic http/rest as http", () => {
		expect(classifyServerTransport("http")).toBe("http");
		expect(classifyServerTransport("rest")).toBe("http");
	});

	test("returns unknown for empty or unrecognized values", () => {
		expect(classifyServerTransport(null)).toBe("unknown");
		expect(classifyServerTransport("")).toBe("unknown");
		expect(classifyServerTransport("websocket")).toBe("unknown");
	});
});

describe("inferServerInstallKind", () => {
	test("maps classified transports onto install draft kinds", () => {
		expect(inferServerInstallKind("sse")).toBe("sse");
		expect(inferServerInstallKind("streamable_http")).toBe("streamable_http");
		expect(inferServerInstallKind("http")).toBe("streamable_http");
		expect(inferServerInstallKind("stdio")).toBe("stdio");
		expect(inferServerInstallKind("websocket")).toBe("stdio");
	});
});

describe("isRemoteHttpTransport", () => {
	test("treats sse, streamable, and generic http as remote", () => {
		expect(isRemoteHttpTransport("sse")).toBe(true);
		expect(isRemoteHttpTransport("streamable_http")).toBe(true);
		expect(isRemoteHttpTransport("http")).toBe(true);
		expect(isRemoteHttpTransport("stdio")).toBe(false);
		expect(isRemoteHttpTransport("websocket")).toBe(false);
	});
});
