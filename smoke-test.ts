#!/usr/bin/env -S deno run --allow-net --allow-run --allow-env
/**
 * RustySquid Post-Deployment Smoke Test
 *
 * Validates that RustySquid is properly deployed and integrated with the wireless system.
 * Run after deployment to ensure everything is working correctly.
 *
 * Usage: deno run --allow-net --allow-run --allow-env smoke-test.ts
 */

const ROUTER_IP = Deno.env.get("ROUTER_IP") || "192.168.50.1";
const ROUTER_USER = Deno.env.get("ROUTER_USER") || "noah";
const PROXY_PORT = parseInt(Deno.env.get("PROXY_PORT") || "3128");

interface TestResult {
  name: string;
  passed: boolean;
  message: string;
  details?: unknown;
}

class SmokeTest {
  private results: TestResult[] = [];
  private failureCount = 0;

  async run(): Promise<void> {
    console.log("🔍 RustySquid Post-Deployment Smoke Test\n");
    console.log(`Router: ${ROUTER_IP}`);
    console.log(`Proxy Port: ${PROXY_PORT}\n`);

    // Run all tests
    await this.testProxyProcess();
    await this.testProxyPort();
    await this.testIptablesRules();
    await this.testProxyConnectivity();
    await this.testCacheFunctionality();
    await this.testTransparentProxy();
    await this.testPerformanceBaseline();

    // Print summary
    this.printSummary();

    // Exit with appropriate code
    Deno.exit(this.failureCount > 0 ? 1 : 0);
  }

  private async execSSH(command: string): Promise<string> {
    const cmd = new Deno.Command("ssh", {
      args: [`${ROUTER_USER}@${ROUTER_IP}`, command],
      stdout: "piped",
      stderr: "piped",
    });

    const output = await cmd.output();
    if (!output.success) {
      const errorText = new TextDecoder().decode(output.stderr);
      throw new Error(errorText || "SSH command failed");
    }

    return new TextDecoder().decode(output.stdout);
  }

  private async testProxyProcess(): Promise<void> {
    const test = "Proxy Process Running";
    try {
      const stdout = await this.execSSH(
        "ps w | grep rustysquid | grep -v grep",
      );

      const lines = stdout.trim().split("\n").filter((l) => l);
      if (lines.length > 0) {
        const pid = lines[0].split(/\s+/)[0];
        this.pass(test, `Process running (PID: ${pid})`);
      } else {
        this.fail(test, "No rustysquid process found");
      }
    } catch (error) {
      this.fail(test, `Failed to check process: ${error}`);
    }
  }

  private async testProxyPort(): Promise<void> {
    const test = "Proxy Port Listening";
    try {
      const stdout = await this.execSSH(`netstat -tunl | grep :${PROXY_PORT}`);

      if (stdout.includes(`:${PROXY_PORT}`)) {
        this.pass(test, `Port ${PROXY_PORT} is listening`);
      } else {
        this.fail(test, `Port ${PROXY_PORT} is not listening`);
      }
    } catch (error) {
      this.fail(test, `Failed to check port: ${error}`);
    }
  }

  private async testIptablesRules(): Promise<void> {
    const test = "Transparent Proxy Rules";
    try {
      const stdout = await this.execSSH(
        `iptables -t nat -L PREROUTING -n | grep ${PROXY_PORT}`,
      );

      const hasHttp = stdout.includes("dpt:80");
      const hasHttps = stdout.includes("dpt:443");

      if (hasHttp && hasHttps) {
        this.pass(test, "HTTP and HTTPS redirection rules active");
      } else if (hasHttp || hasHttps) {
        this.warn(test, `Only ${hasHttp ? "HTTP" : "HTTPS"} rule active`);
      } else {
        this.fail(test, "No transparent proxy rules found");
      }
    } catch (error) {
      this.fail(test, `Failed to check iptables: ${error}`);
    }
  }

  private async testProxyConnectivity(): Promise<void> {
    const test = "Proxy HTTP Request";

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 5000);

      const response = await fetch(`http://httpbin.org/get`, {
        method: "GET",
        signal: controller.signal,
        headers: {
          "Host": "httpbin.org",
          "Proxy-Connection": "Keep-Alive",
        },
        // @ts-ignore - Deno supports proxy in fetch options
        proxy: {
          url: `http://${ROUTER_IP}:${PROXY_PORT}`,
        },
      }).finally(() => clearTimeout(timeoutId));

      if (
        response.status === 200 || response.status === 301 ||
        response.status === 302
      ) {
        this.pass(test, `Proxy responding (HTTP ${response.status})`);
      } else if (response.status === 502) {
        this.warn(test, "Proxy returned 502 (upstream connectivity issue)");
      } else {
        this.fail(test, `Unexpected status code: ${response.status}`);
      }
    } catch (error) {
      // Alternative test using curl through SSH
      try {
        const stdout = await this.execSSH(
          `curl -s -o /dev/null -w "%{http_code}" -x localhost:${PROXY_PORT} http://httpbin.org/get`,
        );
        const statusCode = parseInt(stdout.trim());

        if (statusCode === 200 || statusCode === 301 || statusCode === 302) {
          this.pass(test, `Proxy responding via curl (HTTP ${statusCode})`);
        } else {
          this.fail(test, `Proxy returned ${statusCode}`);
        }
      } catch (curlError) {
        this.fail(test, `Connection failed: ${error}`);
      }
    }
  }

  private async testCacheFunctionality(): Promise<void> {
    const test = "Cache Functionality";
    const testUrl = "http://httpbin.org/cache/3600";

    try {
      // First request - should be a cache MISS
      const start1 = Date.now();
      await this.execSSH(
        `curl -s -o /dev/null -w "%{time_total}" -x localhost:${PROXY_PORT} ${testUrl}`,
      );
      const time1 = Date.now() - start1;

      // Second request - should be a cache HIT (faster)
      const start2 = Date.now();
      await this.execSSH(
        `curl -s -o /dev/null -w "%{time_total}" -x localhost:${PROXY_PORT} ${testUrl}`,
      );
      const time2 = Date.now() - start2;

      if (time2 < time1 * 0.5) {
        this.pass(test, `Cache working (${time1}ms → ${time2}ms)`);
      } else {
        this.warn(test, `Cache may not be working (${time1}ms → ${time2}ms)`);
      }
    } catch (error) {
      this.fail(test, `Failed to test cache: ${error}`);
    }
  }

  private async testTransparentProxy(): Promise<void> {
    const test = "Transparent Proxy Integration";
    try {
      const stdout = await this.execSSH(
        `iptables -t nat -L PREROUTING -n -v | grep ${PROXY_PORT}`,
      );

      // Parse packet counts
      const lines = stdout.trim().split("\n");
      let totalPackets = 0;

      for (const line of lines) {
        const match = line.match(/^\s*(\d+)\s+(\d+)/);
        if (match) {
          totalPackets += parseInt(match[1]);
        }
      }

      if (totalPackets > 0) {
        this.pass(
          test,
          `Traffic flowing (${totalPackets} packets intercepted)`,
        );
      } else {
        this.warn(test, "Rules configured but no traffic intercepted yet");
      }
    } catch (error) {
      this.fail(test, `Failed to check traffic flow: ${error}`);
    }
  }

  private async testPerformanceBaseline(): Promise<void> {
    const test = "Performance Baseline";
    const iterations = 5;
    const times: number[] = [];

    try {
      for (let i = 0; i < iterations; i++) {
        const start = Date.now();
        await this.execSSH(
          `curl -s -o /dev/null -w "%{time_total}" -x localhost:${PROXY_PORT} http://httpbin.org/status/200`,
        );
        times.push(Date.now() - start);
      }

      const avg = times.reduce((a, b) => a + b, 0) / times.length;
      const min = Math.min(...times);
      const max = Math.max(...times);

      if (avg < 500) {
        this.pass(
          test,
          `Good performance (avg: ${
            avg.toFixed(0)
          }ms, min: ${min}ms, max: ${max}ms)`,
        );
      } else if (avg < 1000) {
        this.warn(test, `Moderate performance (avg: ${avg.toFixed(0)}ms)`);
      } else {
        this.fail(test, `Poor performance (avg: ${avg.toFixed(0)}ms)`);
      }
    } catch (error) {
      this.fail(test, `Performance test failed: ${error}`);
    }
  }

  private pass(name: string, message: string): void {
    this.results.push({ name, passed: true, message });
    console.log(`✅ ${name}: ${message}`);
  }

  private fail(name: string, message: string): void {
    this.results.push({ name, passed: false, message });
    this.failureCount++;
    console.log(`❌ ${name}: ${message}`);
  }

  private warn(name: string, message: string): void {
    this.results.push({ name, passed: true, message });
    console.log(`⚠️  ${name}: ${message}`);
  }

  private printSummary(): void {
    console.log("\n" + "=".repeat(50));
    console.log("📊 Test Summary\n");

    const passed = this.results.filter((r) => r.passed).length;
    const total = this.results.length;
    const percentage = ((passed / total) * 100).toFixed(0);

    console.log(`Tests Passed: ${passed}/${total} (${percentage}%)`);

    if (this.failureCount === 0) {
      console.log("\n✅ All critical tests passed! RustySquid is ready.");
    } else {
      console.log(
        `\n❌ ${this.failureCount} test(s) failed. Please review the deployment.`,
      );
    }

    console.log("\n📝 Next Steps:");
    if (this.failureCount === 0) {
      console.log(
        '- Monitor logs: ssh noah@192.168.50.1 "tail -f /tmp/rustysquid.log"',
      );
      console.log(
        "- Check cache stats: curl -x 192.168.50.1:3128 http://httpbin.org/cache/3600",
      );
      console.log(
        '- View connections: ssh noah@192.168.50.1 "netstat -tn | grep 3128"',
      );
    } else {
      console.log("- Check proxy logs for errors");
      console.log("- Verify network connectivity from router");
      console.log("- Ensure iptables rules are not conflicting");
      console.log("- Run: ./deploy_to_router.sh to redeploy");
    }
  }
}

// Run the smoke test
if (import.meta.main) {
  const test = new SmokeTest();
  await test.run();
}
