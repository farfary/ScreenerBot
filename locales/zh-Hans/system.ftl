# Results of system and configuration operations. Ids come from
# src/webserver/routes/config/operations.rs (diff) and
# src/webserver/routes/config/import_export.rs (import).

system-result-config-differs = 内存中的配置与磁盘版本不同
system-result-config-matches = 内存中的配置与磁盘版本一致

# $count is the number of imported sections, $warnings the number of problems and
# $details their text joined with commas.
system-result-config-imported =
    成功导入 { $count ->
       *[other] { $count } 个分区
    }
system-result-config-imported-with-warnings =
    已导入 { $count ->
       *[other] { $count } 个分区
    }，其中有 { $warnings ->
       *[other] { $warnings } 条警告
    }：{ $details }

# The Config page (pages/config.js, config.html and pages/config/*) and the
# import/export dialog (ui/config_import_export_dialog.js). Field labels, hints,
# units and section names come from config.ftl; only the page's own text is here.

## Config page: sidebar and toolbar

system-config-search =
    .placeholder = 搜索设置...
system-config-export-title =
    .title = 将配置导出到文件
system-config-import-title =
    .title = 从文件导入配置
system-config-reload = 从磁盘重新加载
system-config-reset-defaults = 恢复默认值
system-config-select-section = 请选择一个配置分区
system-config-select-section-details = 请选择一个配置分区以查看详情。
system-config-no-metadata = <code>{ $section }</code> 没有元数据
system-config-technical-settings = 技术设置
system-config-expand-title = 展开所有分区及所有嵌套子配置
system-config-collapse-title = 折叠所有分区及所有嵌套子配置
system-config-toolbar-no-changes = 分区内无更改
system-config-toolbar-section-changes =
    { $count ->
       *[other] 分区内有 <strong>{ $count }</strong> 处更改
    }
system-config-toolbar-total-changes =
    { $count ->
       *[other] 共 <strong>{ $count }</strong> 处更改
    }

## Config page: state banner

system-config-loading = 正在加载配置…
system-config-refreshing = 正在刷新配置…
system-config-saving-title = 正在保存更改…
system-config-saving-detail = 正在更新配置
system-config-validation-issues = <strong>检测到验证问题。</strong>请检查高亮的字段。

## Config page: section header and category chips

system-config-save-changes = 保存更改
system-config-saving = 正在保存…
system-config-compare = 与磁盘版本对比
system-config-revert-section = 还原分区
system-config-summary-critical = 关键 { $count } 项
system-config-summary-performance = 性能 { $count } 项
system-config-summary-pending =
    { $count ->
       *[other] { $count } 处待保存更改
    }
system-config-summary-none = 暂无元数据摘要
system-config-fields-count =
    { $count ->
       *[other] { $count } 个字段
    }
# $fields is the field count above; $pending and $visible are counts.
system-config-chip-pending = { $fields } · { $pending } 项待保存
system-config-chip-visible = { $visible } / { $fields }

## Config page: field rows

system-config-field-default = 默认值：{ $value }
system-config-field-reset = 重置为默认值
system-config-array-invalid-title = 数组条目无效
system-config-json-invalid-title = JSON 无效
system-config-list-separator = { "、" }
# Ids of the array-entry messages come from FieldType in src/config/metadata.rs.
# $lines is the list of offending line numbers.
system-config-array-invalid-integer =
    { $count ->
       *[other] 第 { $lines } 行必须是有效的整数。
    }
system-config-array-invalid-number =
    { $count ->
       *[other] 第 { $lines } 行必须是有效的数字。
    }
system-config-array-invalid-boolean =
    { $count ->
       *[other] 第 { $lines } 行必须是有效的布尔值。
    }
system-config-array-invalid-value =
    { $count ->
       *[other] 第 { $lines } 行必须是有效的值。
    }

## Config page: Telegram actions

system-config-telegram-actions = 操作
system-config-telegram-test-title = 测试连接
system-config-telegram-test-description = 发送一条测试消息，以验证您的 { -telegram } 配置是否正常
system-config-telegram-send-test = 发送测试消息
system-config-telegram-sending = 正在发送...
system-config-telegram-configure-token-title = 请先配置机器人令牌
system-config-telegram-configure-token-status = 请先在上方配置机器人令牌以启用测试
system-config-telegram-test-sent-status = 测试消息发送成功！请查看您的 { -telegram }。
system-config-telegram-test-sent = { -telegram } 测试消息已发送
system-config-telegram-test-failed = 发送测试消息失败
system-config-telegram-auth-title = 机器人身份验证
system-config-telegram-totp-title = 双重验证（TOTP）
system-config-telegram-totp-configured = 已配置
system-config-telegram-totp-not-configured = 未配置
system-config-telegram-totp-active = 双重验证已启用。{ -telegram } 会话过期后，需要输入验证器应用中的 TOTP 验证码。
system-config-telegram-totp-inactive = 请在安全设置中启用双重验证，以保护 { -telegram } 命令。
system-config-telegram-totp-note = TOTP 与仪表盘锁屏共用。请在安全设置中配置。
system-config-telegram-require-2fa = 命令需要双重验证
# $status is the HTTP status code.
system-config-telegram-save-rejected = 保存被拒绝（{ $status }）
system-config-telegram-save-failed = 无法保存 { -telegram } 设置

## Config page: operations

system-config-saved = 配置已保存
system-config-save-failed = 无法保存配置
system-config-reloaded = 已从磁盘重新加载配置
system-config-reload-failed = 无法重新加载配置
system-config-diff-title = 配置差异
system-config-diff-console = 已输出到浏览器控制台
system-config-diff-failed = 无法计算差异
system-config-reset-title = 重置配置
system-config-reset-message =
    这将把整个配置重置为内置的默认值。所有当前设置都将丢失。

    此操作无法撤销。
system-config-reset-done-title = 配置已重置
system-config-reset-done-message = 所有设置已恢复为默认值
system-config-reset-failed = 无法重置配置
system-config-load-failed = 无法加载配置
system-config-metadata-failed = 无法加载配置元数据

## Import and export dialogs: shared

system-config-dialog-close =
    .aria-label = 关闭
system-config-select-none = 全不选
system-config-section-gui = 界面
system-config-changes-count =
    { $count ->
       *[other] { $count } 处更改
    }
system-config-sections-count =
    { $count ->
       *[other] { $count } 个分区
    }

## Import and export dialogs: section descriptions. Ids are the section names of
## src/webserver/routes/config/import_export.rs.

system-config-section-hint-chains = 区块链启用、RPC 端点和兑换路由
system-config-section-hint-trader = 交易规则和自动化
system-config-section-hint-positions = 仓位管理设置
system-config-section-hint-filtering = 代币过滤规则和阈值
system-config-section-hint-tokens = 代币发现和数据来源
system-config-section-hint-events = 事件记录设置
system-config-section-hint-services = 后台服务设置
system-config-section-hint-monitoring = 系统监控配置
system-config-section-hint-ohlcv = K 线数据设置
system-config-section-hint-gui = 仪表盘和界面设置
system-config-section-hint-telegram = { -telegram } 机器人配置

## Export dialog

system-config-export-dialog-title = 导出配置
system-config-export-intro = 选择要导出的配置分区。导出的文件之后可导入以恢复或分享设置。
system-config-export-sections = 分区
system-config-export-timestamp = 包含导出时间戳
system-config-sections-selected =
    { $count ->
       *[other] 已选择 { $count } 个分区
    }
system-config-exporting = 正在导出...
system-config-export-invalid-response = 服务器响应无效
system-config-exported-title = 配置已导出
system-config-exported-message =
    { $count ->
       *[other] 已导出 { $count } 个分区
    }
system-config-export-failed-title = 导出失败
system-config-export-failed = 导出配置失败

## Import dialog

system-config-import-dialog-title = 导入配置
system-config-import-upload-intro = 上传之前导出的配置文件。您可以预览并选择要导入的分区。
system-config-import-dropzone-title = 将配置文件拖放到此处
system-config-import-dropzone-hint = 或点击浏览
system-config-import-analyzing = 正在分析配置...
system-config-import-preview = 预览
system-config-import-preview-intro = 请查看下方的配置分区，并选择要导入的分区。
system-config-import-sections = 文件中的分区
system-config-import-select-valid = 选择全部有效项
system-config-import-merge-label = 与现有配置合并
system-config-import-merge-hint = 仅更新文件中存在的字段。取消勾选则替换整个分区。
system-config-import-save-label = 保存到磁盘
system-config-import-save-hint = 导入后将更改写入 config.toml
system-config-import-selected = 导入所选
system-config-import-warnings =
    { $count ->
       *[other] { $count } 条警告
    }
# $section is a section name from the file, $field a dotted setting path, $detail the
# technical reason a section failed to parse.
system-config-import-warning-unknown-section = 未知分区“{ $section }”将被忽略
system-config-import-warning-sensitive-field = 导入 { $field } 可能会覆盖身份验证设置
system-config-import-section-error = { $detail }
# $sections and $changes are the counts above, already worded.
system-config-import-summary = { $sections } • { $changes }
system-config-import-status-absent = 文件中没有
system-config-import-status-invalid = 配置无效
system-config-import-status-unchanged = 无更改
system-config-import-not-included = 文件中未包含
system-config-import-show-changes = 显示更改
system-config-import-hide-changes = 隐藏更改
system-config-import-value-current = 当前值
system-config-import-value-new = 新值
system-config-import-more-changes =
    { $count ->
       *[other] 另有 { $count } 处更改
    }
system-config-import-value-items =
    { "[" }{ $count ->
       *[other] { $count } 项
    }{ "]" }
system-config-import-value-keys =
    { "{" }{ $count ->
       *[other] { $count } 个键
    }{ "}" }
system-config-importing = 正在导入...
system-config-import-failed = 导入失败
system-config-import-invalid-file-title = 文件无效
system-config-import-invalid-file = 解析配置文件失败
system-config-imported-title = 配置已导入
system-config-imported-message =
    { $count ->
       *[other] 已导入 { $count } 个分区
    }
system-config-import-failed-title = 导入失败
system-config-import-failed-message = 导入配置失败
