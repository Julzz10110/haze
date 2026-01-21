/**
 * Load test script for HAZE - sends many transactions via SDK
 *
 * Features:
 * - Configurable transaction rate (tx/sec)
 * - Multiple concurrent senders
 * - Statistics collection (success rate, latency)
 * - Can target multiple nodes (round-robin)
 *
 * Usage:
 *   HAZE_LOAD_NODE_URLS="http://127.0.0.1:8080,http://127.0.0.1:8081" \
 *   HAZE_LOAD_TX_COUNT=1000 \
 *   HAZE_LOAD_TX_PER_SEC=50 \
 *   HAZE_LOAD_CONCURRENT=5 \
 *   node dist/examples/load-test.js
 */

import { HazeClient } from '../src/client';
import { KeyPair, TransactionBuilder, DEFAULT_API_URL } from '../src/index';

interface LoadTestConfig {
  nodeUrls: string[];
  txCount: number;
  txPerSec: number;
  concurrent: number;
}

interface Stats {
  sent: number;
  success: number;
  failed: number;
  latencies: number[];
  startTime: number;
  endTime?: number;
}

function parseConfig(): LoadTestConfig {
  const nodeUrlsEnv = process.env.HAZE_LOAD_NODE_URLS;
  const nodeUrls = nodeUrlsEnv
    ? nodeUrlsEnv.split(',').map((s) => s.trim()).filter(Boolean)
    : [DEFAULT_API_URL];

  const txCount = Number(process.env.HAZE_LOAD_TX_COUNT ?? '100');
  const txPerSec = Number(process.env.HAZE_LOAD_TX_PER_SEC ?? '10');
  const concurrent = Number(process.env.HAZE_LOAD_CONCURRENT ?? '1');

  return {
    nodeUrls,
    txCount: Math.max(1, txCount),
    txPerSec: Math.max(1, txPerSec),
    concurrent: Math.max(1, Math.min(concurrent, 50)), // Cap at 50 concurrent
  };
}

function createClients(nodeUrls: string[]): HazeClient[] {
  return nodeUrls.map((url) => new HazeClient({ baseUrl: url }));
}

async function sendTransactionBatch(
  clients: HazeClient[],
  sender: KeyPair,
  recipient: KeyPair,
  startNonce: number,
  count: number,
  delayMs: number,
  stats: Stats,
): Promise<void> {
  const recipientAddr = recipient.getAddress();

  for (let i = 0; i < count; i++) {
    const nonce = startNonce + i;
    const client = clients[nonce % clients.length]; // Round-robin

    try {
      const tx = TransactionBuilder.createTransfer(
        sender.getAddress(),
        recipientAddr,
        BigInt(1), // Minimal amount
        BigInt(1), // Minimal fee
        nonce,
      );

      const signed = await TransactionBuilder.sign(tx, sender);
      const start = Date.now();
      await client.sendTransaction(signed);
      const latency = Date.now() - start;

      stats.success++;
      stats.latencies.push(latency);
    } catch (err: any) {
      stats.failed++;
      // Log first few errors, then suppress
      if (stats.failed <= 5) {
        console.warn(`Tx #${nonce} failed:`, err?.message ?? err);
      }
    }

    stats.sent++;

    // Rate limiting: delay between transactions
    if (delayMs > 0 && i < count - 1) {
      await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
  }
}

async function runLoadTest(config: LoadTestConfig): Promise<void> {
  console.log('HAZE Load Test');
  console.log('==============');
  console.log(`Nodes: ${config.nodeUrls.join(', ')}`);
  console.log(`Total transactions: ${config.txCount}`);
  console.log(`Target rate: ${config.txPerSec} tx/sec`);
  console.log(`Concurrent senders: ${config.concurrent}`);
  console.log();

  const clients = createClients(config.nodeUrls);
  const stats: Stats = {
    sent: 0,
    success: 0,
    failed: 0,
    latencies: [],
    startTime: Date.now(),
  };

  // Generate keypairs
  const sender = await KeyPair.generate();
  const recipient = await KeyPair.generate();

  // Calculate delay between transactions to achieve target rate
  const delayMs = config.txPerSec > 0 ? 1000 / config.txPerSec : 0;
  const txPerSender = Math.ceil(config.txCount / config.concurrent);

  console.log(`Starting load test...`);
  console.log(`Delay between tx: ${delayMs.toFixed(2)}ms`);
  console.log();

  // Start concurrent senders
  const promises: Promise<void>[] = [];
  for (let i = 0; i < config.concurrent; i++) {
    const startNonce = i * txPerSender;
    const count = Math.min(txPerSender, config.txCount - startNonce);
    if (count > 0) {
      promises.push(
        sendTransactionBatch(clients, sender, recipient, startNonce, count, delayMs, stats),
      );
    }
  }

  // Wait for all senders to complete
  await Promise.all(promises);

  stats.endTime = Date.now();
  const duration = (stats.endTime - stats.startTime) / 1000;

  // Print statistics
  console.log();
  console.log('Load Test Results');
  console.log('=================');
  console.log(`Duration: ${duration.toFixed(2)}s`);
  console.log(`Total sent: ${stats.sent}`);
  console.log(`Success: ${stats.success}`);
  console.log(`Failed: ${stats.failed}`);
  console.log(`Success rate: ${((stats.success / stats.sent) * 100).toFixed(2)}%`);

  if (stats.latencies.length > 0) {
    const sorted = [...stats.latencies].sort((a, b) => a - b);
    const p50 = sorted[Math.floor(sorted.length * 0.5)];
    const p95 = sorted[Math.floor(sorted.length * 0.95)];
    const p99 = sorted[Math.floor(sorted.length * 0.99)];
    const avg = sorted.reduce((a, b) => a + b, 0) / sorted.length;

    console.log();
    console.log('Latency (ms):');
    console.log(`  Average: ${avg.toFixed(2)}`);
    console.log(`  P50: ${p50}`);
    console.log(`  P95: ${p95}`);
    console.log(`  P99: ${p99}`);
    console.log(`  Min: ${Math.min(...sorted)}`);
    console.log(`  Max: ${Math.max(...sorted)}`);
  }

  const actualRate = stats.sent / duration;
  console.log();
  console.log(`Actual rate: ${actualRate.toFixed(2)} tx/sec`);
  console.log(`Target rate: ${config.txPerSec} tx/sec`);
}

async function main() {
  try {
    const config = parseConfig();
    await runLoadTest(config);
  } catch (error: any) {
    console.error('Load test failed:', error?.message ?? error);
    if (error.stack) {
      console.error(error.stack);
    }
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}
