import { describe, expect, test } from "bun:test";
import { normalizeDesktopPlatform, readTauriAdminDiscoveryPlatform } from "./desktop-platform";

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
});
