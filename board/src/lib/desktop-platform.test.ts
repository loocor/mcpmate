import { describe, expect, test } from "bun:test";
import {
	normalizeDesktopPlatform,
	readAdminDiscoveryPlatform,
	readBrowserAdminDiscoveryPlatform,
	readTauriAdminDiscoveryPlatform,
} from "./desktop-platform";

describe("desktop platform helper", () => {
	test("normalizes only Admin discovery supported platforms", () => {
		expect(normalizeDesktopPlatform("macos")).toBe("macos");
		expect(normalizeDesktopPlatform("windows")).toBe("windows");
		expect(normalizeDesktopPlatform("linux")).toBe("linux");
		expect(normalizeDesktopPlatform("darwin")).toBeUndefined();
		expect(normalizeDesktopPlatform("freebsd")).toBeUndefined();
		expect(normalizeDesktopPlatform(null)).toBeUndefined();
	});

	test("reads platform only from an explicit Tauri invoke source", async () => {
		const commands: string[] = [];
		const platform = await readTauriAdminDiscoveryPlatform({
			isTauri: () => true,
			invoke: async (command) => {
				commands.push(command);
				return "macos";
			},
		});

		expect(platform).toBe("macos");
		expect(commands).toEqual(["mcp_shell_read_platform"]);
	});

	test("does not infer platform outside Tauri", async () => {
		const platform = await readTauriAdminDiscoveryPlatform({
			isTauri: () => false,
			invoke: async () => {
				throw new Error("invoke should not be called");
			},
		});

		expect(platform).toBeUndefined();
	});

	test("infers Admin discovery platform from browser navigator values", () => {
		expect(readBrowserAdminDiscoveryPlatform({ userAgentData: { platform: "macOS" } })).toBe("macos");
		expect(readBrowserAdminDiscoveryPlatform({ platform: "Win32" })).toBe("windows");
		expect(readBrowserAdminDiscoveryPlatform({ userAgent: "Mozilla/5.0 (X11; Linux x86_64)" })).toBe("linux");
		expect(readBrowserAdminDiscoveryPlatform({ platform: "FreeBSD" })).toBeUndefined();
	});

	test("uses browser platform outside Tauri for Admin discovery", async () => {
		const platform = await readAdminDiscoveryPlatform({
			isTauri: () => false,
			invoke: async () => {
				throw new Error("invoke should not be called");
			},
			navigatorLike: { platform: "MacIntel" },
		});

		expect(platform).toBe("macos");
	});

	test("prefers explicit Tauri platform over browser platform", async () => {
		const platform = await readAdminDiscoveryPlatform({
			isTauri: () => true,
			invoke: async () => "windows",
			navigatorLike: { platform: "MacIntel" },
		});

		expect(platform).toBe("windows");
	});
});
