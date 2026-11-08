# 模块说明模板

复制以下字段为每个新增或被拆分出的模块写一份说明，并链接到
[`MODULAR_DESIGN_POLICY.md`](MODULAR_DESIGN_POLICY.md)。说明描述职责与契约，
不是验证记录。

```markdown
## <模块名>

- **Path / entry:** <源文件路径与其对外入口>
- **Responsibility:** <这个模块唯一的业务职责，一句话>
- **Owner:** Cross-platform | Windows Platform
- **Depends on:** <它使用的模块/crate/平台适配>
- **Depended on by:** <调用方>
- **Public contract:** <对外类型、函数、序列化格式；明确"不变"或"变更"并链接 ADR>
- **Compatibility entry:** <原文件/旧路径保留的兼容导出，如果没有写 none>
- **State and ownership:** <谁持有 session、owner、permit 或 writer 生命周期>
- **Failure behaviour:** <失败时的 fail-closed 行为与错误码>
- **Tests:** <覆盖它的测试位置>
- **Gates to re-run:** <受影响的 V1 gate 或验证范围>
- **Exceptions:** <architecture-exceptions.json 中的对应条目，如果没有写 none>
- **Open questions:** <尚未决定的事项，或 none>
```

## 填写要求

- **Responsibility** 只能有一句；写出两个职责通常意味着还需要再拆。
- **Public contract** 若发生变化，必须链接 ADR 或标注
  `CROSS_PLATFORM_CHANGE_REQUIRED`。
- **Gates to re-run** 必须引用具体 gate 标识，不写"全部测试"。
- **Exceptions** 与 `scripts/architecture-exceptions.json` 保持一致；不一致时以
  JSON 为准并修正说明。
