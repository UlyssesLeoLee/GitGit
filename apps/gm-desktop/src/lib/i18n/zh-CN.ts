/**
 * 简体中文（默认）文案目录。
 *
 * 文案应保持简短（≤ 32 字符），方便在 980px 宽的窗口里
 * 不会破坏布局。专业术语（如 `repo`, `vault`, `restore`）
 * 直接沿用英文，避免翻译歧义。
 */

const zh: Readonly<Record<string, string>> = {
  // 通用
  'common.appName': 'gitgit Desktop',
  'common.version': '版本',
  'common.loading': '加载中…',
  'common.empty': '暂无数据',
  'common.cancel': '取消',
  'common.confirm': '确认',
  'common.save': '保存',
  'common.delete': '删除',
  'common.refresh': '刷新',
  'common.copy': '复制',
  'common.search': '搜索',
  'common.copiedToClipboard': '已复制到剪贴板',
  'common.yes': '是',
  'common.no': '否',
  'common.close': '关闭',
  'common.undo': '撤销',

  // 侧边栏 / 路由
  'nav.dashboard': '概览',
  'nav.repos': '仓库',
  'nav.vault': '凭证',
  'nav.settings': '设置',

  // 概览 / 服务器
  'dashboard.heading': '服务总览',
  'dashboard.start': '启动服务',
  'dashboard.stop': '停止服务',
  'dashboard.starting': '正在启动…',
  'dashboard.stopping': '正在停止…',
  'dashboard.running': '运行中',
  'dashboard.stopped': '已停止',
  'dashboard.uptime': '已运行',
  'dashboard.pid': 'PID',
  'dashboard.port': '端口',
  'dashboard.bind': '绑定地址',
  'dashboard.recentLogs': '最近日志',
  'dashboard.diagnostics': '诊断信息',
  'dashboard.defaultBindHint': '默认绑定 127.0.0.1:38080（per ADR-0020 §2.2）',

  // 仓库
  'repos.heading': '仓库列表',
  'repos.countOne': '共 {n} 个仓库',
  'repos.sizeBytes': '{n} 字节',
  'repos.sizeKb': '{n} KB',
  'repos.sizeMb': '{n} MB',
  'repos.sizeGb': '{n} GB',
  'repos.defaultBranch': '默认分支',
  'repos.openInFinder': '在文件管理器中打开',
  'repos.cloneUrl': '克隆 URL',
  'repos.copyCloneUrl': '复制克隆 URL',
  'repos.detail': '详情',
  'repos.back': '返回仓库列表',
  'repos.refsHeading': 'Refs',
  'repos.commitsHeading': '最近提交',
  'repos.branchGraph': '分支图谱',
  'repos.refs.none': '该仓库还没有 ref',
  'repos.commits.none': '该仓库还没有提交',
  'repos.notFound': '找不到仓库 {name}',

  // 凭证
  'vault.heading': '凭证管理',
  'vault.subhead': 'FileVault 后端·与 gitai key CLI 等价',
  'vault.add': '新增',
  'vault.keyLabel': '键名',
  'vault.valueLabel': '值',
  'vault.changeNote': '变更备注（可选）',
  'vault.actions': '操作',
  'vault.versions': '历史版本',
  'vault.diff': '对比',
  'vault.restore': '还原',
  'vault.rotate': '轮换',
  'vault.secretCreated': '已写入并记录 v{n}',
  'vault.diffTitle': '对比 v{base} → v{head}',
  'vault.objectChanged': '对象已变更',
  'vault.sizeDelta': '字节差',
  'vault.confirmRestore': '确认把 {key} 还原到 v{n} ？',
  'vault.confirmDelete': '确认删除 {key} ？',
  'vault.empty': '尚未存储任何凭证',

  // 设置
  'settings.heading': '设置',
  'settings.theme': '主题',
  'settings.themeLight': '浅色',
  'settings.themeDark': '深色',
  'settings.themeAuto': '跟随系统',
  'settings.locale': '语言',
  'settings.adminPassword': '管理员密码',
  'settings.adminPasswordSet': '已设置（长度 {n}）',
  'settings.adminPasswordNotSet': '未设置（当前使用默认值）',
  'settings.adminPasswordUpdate': '更新密码',
  'settings.adminPasswordClear': '清除密码',
  'settings.dataDir': '数据目录',
  'settings.about': '关于',
  'settings.updates.title': '自动更新',
  'settings.updates.checkOnStartup': '启动时检查更新',
  'settings.updates.placeholderNote': '（占位）本期不推送更新；通道与签名在 V1 配置',

  // 错误边界
  'errors.bootFailedTitle': '应用启动失败',
  'errors.bootFailedHint': '请通过 Settings → 诊断或查看最近日志定位。',
  'errors.routeTitle': '页面加载失败',
  'errors.routeRetry': '重试',
  'errors.kind.ServerAlreadyRunning': '服务已经在运行（pid={n}）',
  'errors.kind.ServerNotRunning': '服务尚未运行',
  'errors.kind.Bind': '无法绑定端口',
  'errors.kind.Gitgit': 'gitgit 内部错误',
  'errors.kind.Git': 'git 子进程失败',
  'errors.kind.Io': '本地文件系统错误',
  'errors.kind.Vault': '凭证仓错误',
  'errors.kind.Bridge': '系统桥接错误',
  'errors.kind.Internal': '内部错误',
  'errors.kind.InvalidRepoName': '仓库名非法',
};

export default zh;
