# Phase L 参考实现（canonical — 供 diff，勿覆盖你的 src）

这些是 lesson L1–L3 展示代码的**完整、已实测**版本。在 job 沙箱里跑过：
`cargo test --all` = **15 测试全绿**（7 单元 + 8 集成）、`cargo clippy -- -D warnings` 干净、`cargo fmt --check` 干净。

> 教学纪律：引擎代码你**亲手敲**。这里是对照答案，用来 diff 你的实现，不是拿来覆盖 `src/`。

## 直接可落地（manifest 栈，与你的真引擎正交）

| 文件 | 对应 | 说明 |
|---|---|---|
| `log.rs` | L1 | 共享帧 `crc+len+payload` + torn-tail 容错。`wal.rs` 重构成它的 KV 客户、`manifest.rs` 是它的 VersionEdit 客户。可逐字落地。 |
| `version_edit.rs` | L1 | tag 化 `VersionEdit` 编解码。可逐字落地。 |
| `version.rs` | L2 | `Version` 折叠（add/delete 双分支）+ 快照。可逐字落地。 |
| `manifest.rs` | L2 | `recover` + 7 步原子 `install` + `append_edit`。可逐字落地。 |

## 示意（需适配你的真引擎）

| 文件 | 说明 |
|---|---|
| `engine.illustrative.rs` | L3 的 `open`/`flush` 集成逻辑**忠实**，但沙箱为聚焦 manifest 用了**简化的 plain-KV memtable + 扁平 SST**，且省了 `imm` 层。落到你的真引擎时：`mem`/`ssts` 用你 H–K 的 InternalKey 版本；`flush` 排空 `imm` 全部（一条 edit 的 `new_files` 收多个文件号）。**manifest 相关的三段——open 第 2/4 步、flush 的 SST→edit→截 WAL 排序、`next_sst`→`next_file_number`——逐字对应。** |
| `tests.illustrative.rs` | L2 崩溃恢复（a/b1/b2/c）+ L3 孤儿守卫。崩溃恢复部分对真引擎逐字可用；孤儿/重开部分依赖上面的简化 engine，按你的引擎签名微调。 |

## 沙箱 MVP 简化（诚实标注，对应后续 phase）

- **旧 MANIFEST 不删**（轮换后累积）→ Phase X 文件 GC（`DeleteObsoleteFiles`）。
- **孤儿 SST 不清理**（崩在 SST 写完、edit 没记之间）→ 同上；本期靠 WAL 兜底数据无损。
- **WAL 整截**（非 log number 标记法）→ Phase X。
- **VersionEdit 数值用定长 u64**（非 varint）；**无 level / key 区间字段** → Phase M 加 tag。
- **号序 = 新旧**的假设只在扁平期成立 → Phase M 分层后被打破。
