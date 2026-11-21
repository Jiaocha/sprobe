# 部署与运维资料

本目录预留给 Agent 和 Console 的部署资产，当前只有占位目录。

| 目录 | 用途 | 对应文档 |
| --- | --- | --- |
| `systemd/` | Linux systemd unit、tmpfiles、加固选项 | 后端 7.5 |
| `windows/` | Windows 服务安装、事件日志、权限 | 后端 7.6 |
| `docker/` | 容器镜像、compose、只读挂载 | 后端 7.7 |
| `prometheus/` | 抓取配置、告警规则、Grafana 面板 | 后端 8.2、11.4 |

## 原则

- Agent 默认只外连，不开放入站端口；需要 Prometheus 时只绑定回环。
- 配置文件权限 0600/0640，令牌不写入命令行、镜像或日志。
- 容器内 `/proc`、`/sys` 默认是宿主/命名空间口径，部署说明必须写清 scope。
- 部署清单、回滚步骤和校验命令以 [后端文档 7.11](../docs/backend/rust_v1_3.md) 为准。
