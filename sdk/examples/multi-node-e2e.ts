/**
 * Multi-node e2e / load test for HAZE using the SDK.
 *
 * Features:
 * - Connect to N nodes (HTTP APIs)
 * - Optionally send M transfer transactions via one node
 * - Verify that all nodes agree on:
 *   - current_height
 *   - block hashes at each height up to min(common_height)
 *   - state_root / finalized heights via BlockchainInfo
 *
 * Configuration (env vars):
 * - HAZE_E2E_NODE_URLS: comma-separated list of base URLs (default: 3 nodes on 8080/8081/8082)
 * - HAZE_E2E_TX_COUNT: number of transactions to attempt (default: 0 = read-only)
 * - HAZE_E2E_TX_NODE_INDEX: index of node to use for sending tx (default: 0)
 */

import { HazeClient } from '../src/client';
import { KeyPair, TransactionBuilder, DEFAULT_API_URL } from '../src/index';

interface NodeConfig {
  name: string;
  baseUrl: string;
  client: HazeClient;
}

function getNodeUrls(): string[] {
  const env = process.env.HAZE_E2E_NODE_URLS;
  if (env && env.trim().length > 0) {
    return env.split(',').map((s) => s.trim()).filter(Boolean);
  }

  // Default: assume three nodes started by scripts on 8080, 8081, 8082
  const base = DEFAULT_API_URL.replace(/:\d+$/, '');
  return [`${base}:8080`, `${base}:8081`, `${base}:8081`.replace('8081', '8082')];
}

async function createNodes(): Promise<NodeConfig[]> {
  const urls = getNodeUrls();
  return urls.map((url, idx) => ({
    name: `node-${idx + 1}`,
    baseUrl: url,
    client: new HazeClient({ baseUrl: url }),
  }));
}

async function healthAndInfo(nodes: NodeConfig[]) {
  console.log('=== Multi-node health & blockchain info ===');
  for (const node of nodes) {
    try {
      const health = await node.client.healthCheck();
      const info = await node.client.getBlockchainInfo();
      console.log(
        `${node.name} (${node.baseUrl}) -> health=${health}, height=${info.current_height}, wave=${info.current_wave}, finalized_height=${info.last_finalized_height}`,
      );
    } catch (err: any) {
      console.error(`${node.name} (${node.baseUrl}) error:`, err?.message ?? err);
    }
  }
  console.log();
}

async function maybeSendTransactions(nodes: NodeConfig[]) {
  const txCount = Number(process.env.HAZE_E2E_TX_COUNT ?? '0');
  if (!Number.isFinite(txCount) || txCount <= 0) {
    console.log('TX phase: skipped (HAZE_E2E_TX_COUNT not set or <= 0)');
    console.log();
    return;
  }

  const txNodeIndex = Number(process.env.HAZE_E2E_TX_NODE_INDEX ?? '0');
  const senderNode = nodes[txNodeIndex] ?? nodes[0];

  console.log(
    `=== Sending up to ${txCount} transfer tx via ${senderNode.name} (${senderNode.baseUrl}) ===`,
  );

  const sender = await KeyPair.generate();
  const recipient = await KeyPair.generate();

  for (let i = 0; i < txCount; i++) {
    try {
      const tx = TransactionBuilder.createTransfer(
        sender.getAddress(),
        recipient.getAddress(),
        BigInt(1), // minimal amount; may still fail due to balance
        BigInt(1),
        i, // nonce
      );
      const signed = await TransactionBuilder.sign(tx, sender);
      const res = await senderNode.client.sendTransaction(signed);
      console.log(`tx #${i} -> hash=${res.hash}, status=${res.status}`);
    } catch (err: any) {
      // For now we only log errors (e.g. insufficient balance) and continue
      console.warn(`tx #${i} failed:`, err?.message ?? err);
    }
  }

  console.log();
}

async function verifyConsensus(nodes: NodeConfig[]) {
  console.log('=== Verifying multi-node consensus (height / blocks / state root) ===');

  // Fetch blockchain info from all nodes
  const infos = await Promise.all(
    nodes.map(async (n) => ({
      node: n,
      info: await n.client.getBlockchainInfo(),
    })),
  );

  // Determine common height (minimum across nodes)
  const minHeight = infos.reduce(
    (min, x) => Math.min(min, x.info.current_height),
    Number.MAX_SAFE_INTEGER,
  );

  console.log(
    'Heights:',
    infos.map((x) => `${x.node.name}=${x.info.current_height}`).join(', '),
  );
  console.log('Common min height:', minHeight);

  if (!Number.isFinite(minHeight) || minHeight === Number.MAX_SAFE_INTEGER) {
    console.log('No height information available, skipping block comparison.');
    return;
  }

  const reference = infos[0];

  // Compare BlockchainInfo state roots / finalized heights
  for (const { node, info } of infos.slice(1)) {
    const sameHeight = info.current_height === reference.info.current_height;
    const sameStateRoot = info.state_root === reference.info.state_root;
    const sameFinalizedHeight =
      info.last_finalized_height === reference.info.last_finalized_height &&
      info.last_finalized_wave === reference.info.last_finalized_wave;

    console.log(
      `${node.name}: height_match=${sameHeight}, state_root_match=${sameStateRoot}, finalized_match=${sameFinalizedHeight}`,
    );
  }

  // Compare block hashes at each height up to common min height
  const maxChecked = Math.min(minHeight, 20); // limit to first 20 heights for speed
  if (maxChecked === 0) {
    console.log('Chain height is 0, skipping per-block comparison.');
    return;
  }

  console.log(`Checking blocks at heights 1..=${maxChecked}`);

  for (let h = 1; h <= maxChecked; h++) {
    const refBlock = await reference.node.client.getBlockByHeight(h);
    for (const { node } of infos.slice(1)) {
      const b = await node.client.getBlockByHeight(h);
      const sameHash = b.hash === refBlock.hash;
      if (!sameHash) {
        console.warn(
          `Mismatch at height ${h} between ${reference.node.name} and ${node.name}: ${refBlock.hash} vs ${b.hash}`,
        );
      }
    }
  }

  console.log('Consensus check completed.');
  console.log();
}

async function main() {
  console.log('HAZE SDK Multi-node e2e / load test\n');

  const nodes = await createNodes();
  console.log(
    'Nodes:',
    nodes.map((n) => `${n.name}=${n.baseUrl}`).join(', '),
  );
  console.log();

  await healthAndInfo(nodes);
  await maybeSendTransactions(nodes);

  // Give the network some time to produce / sync blocks
  const pauseMs = Number(process.env.HAZE_E2E_WAIT_MS ?? '5000');
  if (pauseMs > 0) {
    console.log(`Waiting ${pauseMs}ms before consensus verification...`);
    await new Promise((resolve) => setTimeout(resolve, pauseMs));
    console.log();
  }

  await verifyConsensus(nodes);

  console.log('Multi-node e2e test finished.');
}

if (require.main === module) {
  // eslint-disable-next-line no-console
  main().catch((err) => {
    // eslint-disable-next-line no-console
    console.error(err);
    process.exit(1);
  });
}

