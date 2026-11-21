# 设计图

## 交付图

- [文档功能与前端全景](../assets/Sprobe_文档功能与前端全景.png)：文档功能地图、Agent 运行时链路、Console 信息架构。
- [Console 页面与数据流](../assets/Sprobe_Console_页面与数据流.png)：`/overview`、`/nodes/:id` 线框和只读数据链。
- [图示说明](Sprobe_更新图示.md)：图例、关键口径和与源文档的对应关系。

## 图源

- `src/diagram1.html`
- `src/diagram2.html`

图源是本地 HTML/CSS，使用 Chrome headless 渲染为 2400px 宽的 PNG。修改指标数量、契约字段或前端路由后，需要同时更新图源和说明文档中的数字。

## 注意

图中 JSON、HTTP、配置均为契约示意，不是代码实现；具体字段以 [后端文档](../backend/rust_v1_3.md) 和 [机读契约](../../contract/sprobe_contract_v1_0.schema.json) 为准。
