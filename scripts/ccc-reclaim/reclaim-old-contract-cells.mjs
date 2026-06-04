#!/usr/bin/env node

import * as ccc from "@ckb-ccc/core";

const DEFAULT_TARGETS = [
  {
    name: "sudt-meta-old",
    txHash: "0x5572bf22963955ca96462437ba73f064209354f9476449f540a89a1f4462ca55",
    index: 0,
  },
  {
    name: "xudt-meta-stale",
    txHash: "0xdfe369557dd66b5cb99b030b844e55a5782f4e4abb0c8b94264d51182c53e973",
    index: 0,
  },
];

function parseTargets(raw) {
  if (!raw || !raw.trim()) {
    return DEFAULT_TARGETS;
  }

  return raw
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean)
    .map((item, i) => {
      const [txHash, idxRaw] = item.split(":");
      if (!txHash || idxRaw === undefined) {
        throw new Error(
          `Invalid RECYCLE_OUTPOINTS item at position ${i + 1}: '${item}', expected tx_hash:index`,
        );
      }
      const index = idxRaw.startsWith("0x") ? Number.parseInt(idxRaw, 16) : Number.parseInt(idxRaw, 10);
      if (!Number.isFinite(index) || index < 0) {
        throw new Error(`Invalid outpoint index for '${item}'`);
      }
      return {
        name: `custom-${i + 1}`,
        txHash,
        index,
      };
    });
}

function sameScript(a, b) {
  return (
    a?.codeHash === b?.codeHash &&
    a?.hashType === b?.hashType &&
    a?.args === b?.args
  );
}

function toHexCapacity(capacity) {
  return ccc.numToHex(ccc.numFrom(capacity));
}

async function main() {
  const dryRun = process.argv.includes("--dry-run") || !process.argv.includes("--execute");
  const privKey = process.env.OWNER_PRIVKEY;

  if (!privKey) {
    throw new Error("OWNER_PRIVKEY is required");
  }

  const targets = parseTargets(process.env.RECYCLE_OUTPOINTS);
  const client = new ccc.ClientPublicTestnet();
  const signer = new ccc.SignerCkbPrivateKey(client, privKey);
  const owner = await signer.getRecommendedAddressObj();

  console.log("Reclaim owner lock:", owner.script);
  console.log("Mode:", dryRun ? "dry-run" : "execute");
  console.log("Targets:", targets.map((t) => `${t.name}:${t.txHash}:${t.index}`).join(", "));

  const tx = ccc.Transaction.from({});
  const selected = [];
  let totalSelectedCapacity = ccc.Zero;

  for (const target of targets) {
    const outPoint = { txHash: target.txHash, index: target.index };
    const liveCell = await client.getCellLive(outPoint, true, true);

    if (!liveCell) {
      console.log(`[skip] ${target.name} not live or not found: ${target.txHash}:${target.index}`);
      continue;
    }

    const lock = liveCell.cellOutput.lock;
    if (!sameScript(lock, owner.script)) {
      console.log(`[skip] ${target.name} lock does not match owner: ${target.txHash}:${target.index}`);
      continue;
    }

    if (!liveCell.outputData || liveCell.outputData === "0x") {
      console.log(`[skip] ${target.name} has empty output_data: ${target.txHash}:${target.index}`);
      continue;
    }

    tx.addInput({ previousOutput: outPoint });
    selected.push(target);
    totalSelectedCapacity += ccc.numFrom(liveCell.cellOutput.capacity);

    console.log(
      `[pick] ${target.name} ${target.txHash}:${target.index} capacity=${toHexCapacity(liveCell.cellOutput.capacity)} data_hash=${ccc.hashCkb(liveCell.outputData)}`,
    );
  }

  if (selected.length === 0) {
    console.log("No eligible cells selected, nothing to reclaim.");
    return;
  }

  tx.addOutput(
    {
      capacity: 0,
      lock: owner.script,
    },
    "0x",
  );

  await tx.completeFeeChangeToOutput(signer, 0, undefined, undefined, {
    shouldAddInputs: false,
  });

  const prepared = await signer.signTransaction(tx);
  const outputCapacity = prepared.outputs[0]?.capacity ?? 0;

  console.log("Selected input cells:", selected.length);
  console.log("Total selected capacity:", toHexCapacity(totalSelectedCapacity));
  console.log("Reclaim output capacity:", toHexCapacity(outputCapacity));
  console.log("Prepared tx hash:", prepared.hash());

  if (dryRun) {
    console.log("Dry-run only. Re-run with --execute to send transaction.");
    return;
  }

  const txHash = await signer.sendTransaction(tx);
  console.log("Sent reclaim tx:", txHash);

  await client.waitTransaction(txHash);
  console.log("Transaction committed:", txHash);
}

main().catch((err) => {
  console.error("Reclaim failed:", err?.message ?? err);
  process.exit(1);
});
