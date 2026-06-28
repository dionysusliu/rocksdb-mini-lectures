# Mission: RocksDB 生产级掌握与存储引擎构建

## Why
掌握 RocksDB 这个应用最广泛的 LSM 类单机存储引擎，从生产级部署使用出发，逐步深入架构和源码，最终能够修改源码实现论文中的功能，并构建自己的存储引擎。这是进入存储系统研究和工程实践的关键路径。

## Success looks like
- 能在生产环境中正确部署和调优 RocksDB（配置、compaction、压缩、缓存）
- 能阅读和理解 RocksDB 源码中的核心路径（写入、读取、Compaction）
- 能修改 RocksDB 源码实现新功能（如自定义 Compaction 策略、新增压缩算法）
- 能参考 RocksDB 架构从零实现一个简化版 LSM 存储引擎

## Constraints
- 偏好中文沟通和教学
- 已有 RocksDB 源码在本地（/Users/chuang/Documents/dev/projects/researching/rocksdb/rocksdb-src）
- macOS 开发环境
- 学习路线：使用 → 架构 → 修改源码 → 自建引擎

## Out of scope
- 不涉及分布式一致性协议（Raft/Paxos）的详细实现
- 不涉及 Java 绑定或其他语言绑定
- 不涉及 Windows 平台特定问题
