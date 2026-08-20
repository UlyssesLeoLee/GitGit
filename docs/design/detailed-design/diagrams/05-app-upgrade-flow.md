# App 升级流程

> 详细设计：[`../12-app-registry-and-plugin-loader.md`](../12-app-registry-and-plugin-loader.md) §12.7
> 关联 REQ：APP-REQ-007 (热插拔) / AISEC-REQ-013 (沙箱)
> 关联流程：任务 67 (IT) / 104 (生产环境) / 105 (生产部署)

## Rolling 升级时序图

```mermaid
sequenceDiagram
    actor SRE
    participant Admin as Admin API
    participant Coord as Coordinator
    participant AppInst1 as App Instance 1
    participant AppInst2 as App Instance 2
    participant DB

    SRE->>Admin: POST /apps/{id}/upgrade {version: "1.2.0", strategy: rolling}
    activate Admin
    Admin->>Coord: upgrade(app_id, new_version, strategy)
    activate Coord
    Coord->>DB: BEGIN
    Coord->>DB: INSERT event 'app.upgrade.started'
    Coord->>DB: UPDATE apps SET status='upgrading'
    Coord->>DB: COMMIT

    loop 每个 App instance
        Coord->>AppInst1: GET /health (检查)
        AppInst1-->>Coord: 200 healthy
        Coord->>AppInst1: 标记 instance_id 为 'draining' (停止接收新事件)
        Coord->>DB: 等待 in-flight events 完成
        Coord->>AppInst1: SIGTERM (graceful shutdown)
        AppInst1-->>Coord: 关闭
        Coord->>DB: 启动新 instance (新 version)
        Coord->>DB: 等待 heartbeat healthy
        Coord->>DB: INSERT event 'app.upgrade.partial'
    end

    Coord->>DB: UPDATE apps SET status='enabled', manifest_version='1.2.0'
    Coord->>DB: INSERT event 'app.upgrade.completed'
    deactivate Coord
    Admin-->>SRE: 200 OK (instance 升级完成)
    deactivate Admin
```

## 回滚流程

升级失败 → 自动回滚：
- Coord 检测到新 instance 不健康（> 30s 未 heartbeat）
- 自动启动旧 version 的新 instance
- 删除新 instance
- 写 event `app.upgrade.rolled_back`
- 通知 SRE
