# RocksDB 生产级部署与使用 Resources

## Knowledge

### 阶段 4 自建引擎（Rust mini-LSM）

- [Mini-LSM: LSM in a Week (skyzh, book)](https://skyzh.github.io/mini-lsm/) + [GitHub starter+solution](https://github.com/skyzh/mini-lsm)
  Rust 建 LSM 存储引擎的权威教程，3 周 21 章。用作自建引擎的结构参考底座（用户手写、回扣 RocksDB 源码）。TiDB/CockroachDB 同类引擎。
- [MiniLSM 介绍 (skyzh blog)](https://www.skyzh.dev/blog/2022-12-27-mini-lsm/) + [Rust Magazine 版](https://rustmagazine.org/issue-1/minilsm/)
  教程设计理念与总览。


- [RocksDB 官方 Wiki — Tuning Guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide)
  生产调优的核心参考。涵盖写放大、读放大、空间放大的权衡，MemTable/Block Cache/Compaction 各层配置。

### 并发写设计空间（实时核查 2026-06，配套 concurrent-write-design-space.html）

- [LevelDB Write 源码解析](https://selfboot.cn/en/2025/01/24/leveldb_source_writedb/) + [LevelDB Concurrent Access](http://tonyz93.blogspot.com/2016/11/leveldb-source-reading-4-concurrent.html)
  流派①group commit 源头。writers_ deque + mutex，队头 leader BuildBatchGroup 合并。RocksDB 无锁版的前身。
- [Postgres Fun with LWLocks (Paquier)](https://paquier.xyz/postgresql-2/2023-04-20-fun-with-lwlocks/) + [RFC: Lock-free XLog Reservation](https://www.postgresql.org/message-id/6bd2f1ed-9a48-401f-904e-e4e59102a371@postgrespro.ru)
  流派②并行预留。NUM_XLOGINSERT_LOCKS=8，ReserveXLogInsertLocation 串行预留+并行 memcpy。纯无锁版仍 RFC。
- [Seastar Shared-nothing Design](https://seastar.io/shared-nothing/) + [ScyllaDB Shard-per-Core](https://www.scylladb.com/product/technology/shard-per-core-architecture/)
  流派③shared-nothing。一核一线程独占分片，跨核 lock-free 消息传递，零锁。对自建引擎最有启发。
- [dbdb.io LMDB](https://dbdb.io/db/lmdb) + [Wikipedia LMDB](https://en.wikipedia.org/wiki/Lightning_Memory-Mapped_Database)
  流派④单写线程。SRMW + COW B+tree + 无 WAL。group commit 的反面极端。
- [Binary Log Group Commit in MySQL 5.6 (Thalmann)](http://mysqlmusings.blogspot.com/2012/06/binary-log-group-commit-in-mysql-56.html) + [WL#5223](https://dev.mysql.com/worklog/task/?id=5223)
  流派①变体。Flush/Sync/Commit 三阶段 stage leader，类比 RocksDB pipelined_write。
- [Bw-Tree: A Latch-Free B-Tree (Microsoft Research)](https://www.microsoft.com/en-us/research/publication/bw-tree-latch-free-b-tree-log-structured-flash-storage/) + [Building a Bw-Tree (CMU)](https://db.cs.cmu.edu/papers/2018/mod342-wangA.pdf)
  流派⑤无锁结构。delta record + CAS mapping table，SQL Server Hekaton/Azure DocumentDB 用。
- [MyRocks: A space- and write-optimized MySQL database — Meta Engineering](https://engineering.fb.com/2016/08/31/core-infra/myrocks-a-space-and-write-optimized-mysql-database/)
  Facebook 将 RocksDB 集成为 MySQL 存储引擎的实战报告。展示了 LSM vs B+Tree 在压缩效率、写放大上的对比，以及生产迁移流程。
- [LSM-Tree Database Storage Engine Serving Facebook's Social Graph (MyRocks VLDB 2020)](https://www.vldb.org/pvldb/vol13/p3217-matsunobu.pdf)
  MyRocks 的学术论文版本，详细描述了 MyRocks 在 Facebook 社交图谱中的生产部署细节和性能数据。
- [RocksDB: Evolution of Development Priorities in a Key-value Store (SIGMOD 2021)](https://dl.acm.org/doi/fullHtml/10.1145/3483840)
  RocksDB 核心团队发表的论文，描述了从 LevelDB 演化而来的设计决策、在 Facebook 内部的多种工作负载特征和优化经验。
- [Characterizing, Modeling, and Benchmarking RocksDB Key-Value Workloads (FAST 2020)](https://www.usenix.org/conference/fast20/presentation/cao-zhichao)
  Facebook 三种典型 RocksDB 生产工作负载的特征分析论文，对理解真实负载模式极有价值。
- [Confluent: How to Tune RocksDB for Kafka Streams State Stores](https://www.confluent.io/blog/how-to-tune-rocksdb-kafka-streams-state-stores-performance/)
  Kafka Streams 中 RocksDB 状态存储的调优实践。涵盖 Block Cache、Write Buffer、Compaction 针对流处理场景的配置。
- [TiKV RocksDB Config Documentation](https://tikv.org/docs/3.0/tasks/configure/rocksdb/)
  TiKV 对 RocksDB 的生产配置参考。展示了 Raft Log 和 KV 数据分离为两个 RocksDB 实例的做法。
- [How We Optimize RocksDB in TiKV — SST Compaction Guard](https://medium.com/@siddontang/how-we-optimize-rocksdb-in-tikv-sst-compaction-guard-6c2d2431a7c5)
  TiKV 团队对 RocksDB Compaction 的深度优化案例，展示了生产环境中的真实问题与解法。
- [YugabyteDB: How We Built a High Performance Document Store on RocksDB](https://www.yugabyte.com/blog/how-we-built-a-high-performance-document-store-on-rocksdb/)
  YugabyteDB 在 RocksDB 之上构建文档存储（DocDB）的架构设计，展示了如何将 KV 接口扩展为多模型。
- [YugabyteDB: Enhancing RocksDB for Speed & Scale](https://medium.com/yugabyte/enhancing-rocksdb-for-speed-scale-66ccdcea808b)
  YugabyteDB 对 RocksDB 的性能增强实践，包括数据密度优化和写路径改进。
- [Ceph BlueStore Configuration Reference](https://docs.ceph.com/en/latest/rados/configuration/bluestore-config-ref/)
  Ceph BlueStore 中 RocksDB 的配置参考。展示了元数据存储场景下的 RocksDB 用法。
- [Ceph Blog: RocksDB Compression in Ceph](https://ceph.io/en/news/blog/2025/rocksdb-compression-ftw/)
  Ceph 社区对 RocksDB 压缩在生产中的实测报告，证明压缩无性能损失但有空间收益。
- [Apache Flink: Using RocksDB State Backend](https://flink.apache.org/2021/01/18/using-rocksdb-state-backend-in-apache-flink-when-and-how/)
  Flink 官方对 RocksDB 状态后端的使用指南，涵盖何时选择 RocksDB 以及配置建议。
- [CockroachDB: Introducing Pebble](https://www.cockroachlabs.com/blog/pebble-rocksdb-kv-store/)
  CockroachDB 从 RocksDB 迁移到自研 Pebble 的决策过程。理解为什么生产系统可能需要"逃离" RocksDB。
- [Solana: From ClickHouse to RocksDB](https://www.helius.dev/blog/migrating-from-clickhouse-to-rocksdb)
  Solana 从 ClickHouse 迁移到 RocksDB 作为归档层的经验，展示了非典型但合理的 RocksDB 用法。

## Wisdom (Communities)

- [RocksDB Facebook Group](https://www.facebook.com/groups/rocksdb.dev/)
  官方维护的开发者社区，核心开发者活跃。用于：配置调优问答、版本升级问题、功能请求讨论。
- [TiDB / TiKV Slack](https://tikv.org/docs/contributed/join-the-community/)
  TiKV 社区，大量 RocksDB 生产实践经验。用于：分布式场景下 RocksDB 的深度调优讨论。
- [RocksDB GitHub Issues](https://github.com/facebook/rocksdb/issues)
  官方 issue tracker。用于：bug 报告、功能请求、搜索已知问题。
- [dbdb.io — Databases using RocksDB](https://dbdb.io/browse?embeds=rocksdb)
  所有使用 RocksDB 的数据库索引。用于：发现新项目和参考架构。

## Gaps

- 缺少中国互联网公司（微信 PaxosStore、360 Pika、字节 ByteGraph）的详细技术博客英文翻译
- 缺少 RocksDB 在向量数据库（Qdrant、Milvus）中的详细调优指南
