# 学习偏好与备注

- ★ 教学模式(2026-06-26起)：lab/源码代码全部用户亲手手写，agent 只展示代码+解释为什么，不直接写 .cc/源码。除非用户明确声明让 agent 写。agent 仍可：编译运行验证、改脚手架/验证胶水、修 CMake、review 用户代码。
- 偏好中文沟通和教学
- 学习路线明确：使用 → 架构 → 修改源码 → 自建引擎
- 当前阶段：阶段 3 改源码（Lesson 06 第一次源码修改回路已跑通；阶段 2 架构 L03-05 全完成）
- 下一课：Lesson 07 第一个真正功能修改（候选①CompactionFilter ②自定义选层策略 ③新 statistics）
- 改源码回路已验证：改 .cc → make shared_lib DEBUG_LEVEL=0（增量~8秒）→ lab 动态链接直接生效 → git checkout 回滚
- 源码现状：compaction_picker_level.cc 带 [LAB HACK] 打印（活样本，可 git checkout 清）；compression.cc 有 (void)args 构建修复
- Lab 工作区 lab/ 已建好，01-09 练习全部可编译运行（dylib 路径用 install_name_tool 修好）
- Lab 09 = 用户亲手实现 TTL CompactionFilter（脚手架法），含 Stretch 1+3 完成
- CMake add_lab 已加 -fno-rtti（对齐 release dylib，继承 RocksDB 多态类的 lab 必需）
- reference 库现 5 篇：工业速查/写路径地图/Group Commit/并发设计空间/改源码工作流
- 下一课明确：Lesson 07 改 compaction 选层逻辑做自定义策略（真改源码）
- 教学纪律：性能声明必须实测+标注边界（storage-benchmark-lab），Lab 08 已诚实标注写放大方向依负载
- reference 库现 4 篇：工业项目速查 / 写路径源码地图 / Group Commit 并发模型 / 并发写设计空间
- 重点关注不同类别的顶尖工业项目作为参考
- 本地源码路径：/Users/chuang/Documents/dev/projects/researching/rocksdb/rocksdb-src
- 已有上一轮 session 的基础 API 使用介绍（Put/Get/Delete, Column Family, Transaction, Backup, ldb 工具）

## 阶段 4 自建引擎 lesson 格式约定 (2026-06-26 起)
- ★ 每节课 HTML 必含: ①明确宣称的 GOAL + Done-when 判定 ②编号 steps(写什么代码/跑什么命令) ③要手写的 Rust(展示, 用户敲) ④harness 设计教学(不只"写测试", 教一种可迁移 harness 系统) ⑤留给用户自答的核心问题(不给答案) ⑥一手源 + 追问提醒。
- ★ 目标: 用户处于 flow——目标清晰/知道写什么代码跑什么命令/学会设计 harness 系统。
- 内容全放进课程材料; 太长就拆多 HTML 按层级组织(hub index.html + 子课)。
- agent 写 lesson HTML; 用户手写全部引擎 .rs + 执行全部命令。agent 可在 $CLAUDE_JOB_DIR/tmp 建临时 cargo 工程验证所示代码能编过(不碰 rocksdb-mini/src)。
- 引擎代码仓: /Users/chuang/Documents/dev/projects/researching/rocksdb-mini (Rust, edition 2024; deps: bytes/crossbeam-skiplist/anyhow/tempfile)。lesson 在 teach workspace labs/。
- Lab 0002 = Phase A MemTable (A1 KV类型+比较器 / A2 MemTable点写读 / A3 计量+冻结)。三种 harness 教学: 表驱动(A1)/模型差分oracle(A2)/不变量(A3)。代码已 tmp 验证 4 test 全绿。用户尚未手做。

## 研究 backlog (用户标记日后特别研究)
- ★ 并发 MemTable / LSM 写并发模型 (2026-06-26 标记)：&self vs &mut、无锁跳表 CAS+epoch、RocksDB ①group commit + ⑤无锁跳表分层、seq 给并发同 key 定序(InternalKey)、0007 五流派横向对比。Phase B(WAL 必须串行那段)是天然切入点,届时可深挖,可能产出 reference HTML 或独立 deep-dive lab。详见 memory `research-backlog-concurrent-memtable`。

## Phase B WAL lab (Lab 0003) 已建 (2026-06-26)
- index.html + b1-wal-record-format(record帧:crc32+len+payload, Writer, 故障注入harness) + b2-reader-recovery(replay容忍残尾, Engine{open/put/get}, 崩溃恢复harness)。
- 新 harness 系统: ④故障注入(翻字节/crc必抓) ⑤崩溃恢复(drop+reopen / 截尾torn-write)。累计5种harness(表驱动/模型差分/不变量/故障注入/崩溃恢复)。
- 新增引擎文件: src/wal.rs + src/engine.rs; Cargo.toml 加 crc32fast="1"。Engine::put 是 &mut self(WAL串行点)——接住并发backlog课题, group commit 是后续优化。
- MVP简化(诚实标注): 砍掉 RocksDB 32KB block+record分片(kFirst/Middle/Last) → 无法中间损坏后重同步, 只能尾部容错; u32定长非varint; 每次put都fsync(group commit反面教材)。
- 全部代码已 $CLAUDE_JOB_DIR/tmp 验证: 4 test 全绿(roundtrip/corruption/survives_reopen/truncated_tail)。用户尚未手做。
- 下一: Phase C SST(把冻结MemTable倒成有序SST, A3"满"信号=flush触发器)。

## Phase C SST lab (Lab 0004) + 总 roadmap 已建 (2026-06-26)
- labs/roadmap.html = 总路线图: ~36 phase / 6 tier / 6 里程碑(M1 MVP..M6 自己的RocksDB)。A✓B✓C(now)→D触达M1。活文档随进度更新状态(done/now/todo)。
- Lab 0004: index + c1-sst-builder(布局data/index/footer+SstBuilder+升序不变量harness) + c2-sst-reader(footer→index→二分get+round-trip/差分harness) + c3-flush(Engine::flush倒表+WAL reset+跨持久化边界harness)。
- 新 harness 系统: ⑥格式稳定性/round-trip ⑦跨持久化边界round-trip。累计7种。
- 新增引擎文件 src/sst.rs; 扩展 memtable.rs(加有序iter()) + engine.rs(dir/next_sst字段+flush()) + wal.rs(reset())。
- MVP简化(诚实标注): 全量索引(非稀疏块索引) / 整文件入内存(非block读+cache) / 无bloom/无压缩/无block checksum / flush后直接截断WAL(非manifest+log number)。对应后续 Phase E/R/S/L/X。
- 故意空洞: Engine::get 仍只查MemTable, flush后数据在SST却读不到 → Phase D 补读路径链(active→immutable→SST新到旧)触达M1。
- 全部代码 $CLAUDE_JOB_DIR/tmp/refimpl 累积验证(A+B+C集成): 6 test 全绿(rejects_unsorted/build_open_roundtrip/matches_btreemap_oracle/survives_reopen/flush_produces_correct_sst/wal_truncated_after_flush)。
- 隔离方式: worktree建不了(rocksdb-mini无commit, HEAD不存在), 改用 job-tmp refimpl 沙箱做累积验证 + lab写teach仓, 完全不碰用户 rocksdb-mini/src。

## lesson 格式升级: 五块 (2026-06-26)
- 每课除 🎯Goal+代码 / 🧪Harness 外,新增三支柱: 🧰Coding style(可迁移编程范式) / 🧠System mindset(跨域 Linux/存储/网络/算法/HPC 系统思维) / 🔄CI/CD(逐课搭真流水线)。
- roadmap.html 加"每节课五块"+ 三条横切学习弧表(System mindset 跨域映射 / Coding style / CI/CD)。
- Phase C 三课全回填三支柱。CI/CD 增量: C1 test job + git/gh bootstrap步骤 / C2 fmt+clippy -D warnings gate / C3 rust-cache+完整yml。
- 真 lint 教学素材: clippy 抓到 sst.rs add() collapsible_if → 改 edition2024 let-chain 折叠(C1代码已用,CI干净)。
- ci-reference.yml 落在 labs/0004-phase-c-sst/, 本地验证: YAML合法 + cargo test --all 6绿 + fmt --check干净 + clippy -D warnings干净。
- CI真部署需用户 push rocksdb-mini 到 GitHub(现无remote/commit); C1给一键 git init+gh repo create 步骤。

## Phase D 读路径合并 (Lab 0005) — 多 agent 流程 (2026-06-26)
- 流程: writer subagent 编写 → teacher agent 7点rubric批判 → APPROVE → 2处非阻塞polish → 收工。worktree 隔离, 通知用户 merge。
- D1 读路径接SST(ssts:Vec<SstReader>新到旧, get find_map, open discover_ssts扫盘+next_sst=max+1) — 顺手修真bug(原open把next_sst设0→重开flush覆盖000000.sst丢数据)。
- D2 immutable层(imm:Vec<Arc<MemTable>>, freeze, 三层get active→imm→SST, flush排空全部内存→多SST→整截WAL, flush签名PathBuf→Vec<PathBuf>)。旗舰harness=分层查找差分(3000随机put/freeze/flush vs 扁平BTreeMap oracle+drop/reopen)。
- D3 满了自动flush(put内同步落盘, 接A3 is_full) → 达成 M1 MVP。
- RocksDB锚(核实): column_family.h:207 SuperVersion(mem/imm/current) · db_impl.cc:3351/3365/3407 GetImpl三段式 · version_set.cc:2918 Version::Get。
- System mindset新映射: RCU↔Arc+freeze · dirty-page-writeback↔同步flush · OverlayFS whiteout↔未来tombstone。
- 验证: cargo test --all 10绿 / fmt clean / clippy -D warnings clean。reference/*.rs 与验证工程逐字节相同。
- worktree工件(非破坏性, 未碰src/): reference/(全引擎+README "canonical, not built, diff don't overwrite") + .github/workflows/ci.yml(test/integration/lint)。branch worktree-phase-d-readpath commit a2288c6。
- 待用户 merge。lesson在teach仓working tree(researching/rocksdb/labs/0005)。

## Roadmap 增补 (2026-06-27): T4★ 线程模型 + 性能 A/B
- 用户要求:后期引入 RocksDB 多线程模型,并与 M1 起的单线程同步模型做性能对比。
- 落点:Tier 4 末尾新增 capstone 行 `T4★`(不 re-letter X–AI,避开顶部学习弧对 AF 的引用)。
  ① 同步内联 flush/compaction → 后台线程池(flush/compaction worker + write-thread leader/follower)。
  ② ★ 单线程同步 vs 多线程 基准 A/B:吞吐 / p99 / 写停顿 三指标。
- 锚:db_impl_compaction_flush.cc · write_thread.cc(JoinBatchGroup) · env_posix 线程池。
- harness:并发 oracle + 性能 A/B 回归 gate。M4 milestone + Tier4 tier-sub 同步更新。
