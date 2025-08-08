import { assertEquals, assertExists } from "https://deno.land/std@0.224.0/assert/mod.ts";

Deno.test("Smoke test configuration", () => {
  const ROUTER_IP = Deno.env.get("ROUTER_IP") || "192.168.50.1";
  const ROUTER_USER = Deno.env.get("ROUTER_USER") || "noah";
  const PROXY_PORT = parseInt(Deno.env.get("PROXY_PORT") || "3128");
  
  assertExists(ROUTER_IP, "Router IP should be defined");
  assertExists(ROUTER_USER, "Router user should be defined");
  assertEquals(typeof PROXY_PORT, "number", "Proxy port should be a number");
  assertEquals(PROXY_PORT, 3128, "Default proxy port should be 3128");
});

Deno.test("SSH command construction", async () => {
  const ROUTER_USER = "noah";
  const ROUTER_IP = "192.168.50.1";
  const testCommand = "echo test";
  
  const cmd = new Deno.Command("ssh", {
    args: [`${ROUTER_USER}@${ROUTER_IP}`, testCommand],
    stdout: "piped",
    stderr: "piped",
  });
  
  assertExists(cmd, "SSH command should be constructable");
});

Deno.test("Test result structure", () => {
  interface TestResult {
    name: string;
    passed: boolean;
    message: string;
    details?: unknown;
  }
  
  const result: TestResult = {
    name: "Test",
    passed: true,
    message: "Test passed",
  };
  
  assertEquals(result.name, "Test");
  assertEquals(result.passed, true);
  assertEquals(result.message, "Test passed");
});

Deno.test("Packet parsing regex", () => {
  const line = "    42     4245 REDIRECT   tcp  --  br-lan *       0.0.0.0/0";
  const match = line.match(/^\s*(\d+)\s+(\d+)/);
  
  assertExists(match, "Regex should match iptables output");
  assertEquals(match[1], "42", "Should extract packet count");
  assertEquals(match[2], "4245", "Should extract byte count");
});

Deno.test("Performance calculation", () => {
  const times = [100, 150, 120, 130, 140];
  const avg = times.reduce((a, b) => a + b, 0) / times.length;
  const min = Math.min(...times);
  const max = Math.max(...times);
  
  assertEquals(avg, 128, "Average should be calculated correctly");
  assertEquals(min, 100, "Min should be found correctly");
  assertEquals(max, 150, "Max should be found correctly");
});