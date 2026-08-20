-- 0014 — 初始 seed 数据 (开发环境)
-- 生产环境应使用 secret 管理 + 控制台初始化
INSERT INTO permissions (action, resource_type, description) VALUES
    ('repo.read', 'repository', '读取仓库'),
    ('repo.write', 'repository', '推送 / 写仓库'),
    ('repo.admin', 'repository', '仓库管理'),
    ('graph.read', 'graph', '查询图'),
    ('graph.write', 'graph', '写入图节点 / 边'),
    ('agent.run', 'agent', '运行 Agent'),
    ('agent.approve', 'agent', '批准 Agent 输出'),
    ('admin.app.install', 'app', '安装 App'),
    ('admin.user.manage', 'user', '用户管理')
ON CONFLICT DO NOTHING;
