# 选定协议的官方补核

> 非规范性研究。访问日期2026-10-03；动态网页无可确认发布版本，不用抓取时间代替设计时间。未调用任何模型或凭据服务。Q13首批范围不因网页新增功能自动扩大。

## WEB-01 OpenRouter 图片

[官方图片指南](https://openrouter.ai/docs/guides/overview/multimodal/image-generation)现描述专用 `/api/v1/images`，返回base64媒体与可用usage，另有endpoint能力及流式协议。[官方Chat API](https://openrouter.ai/docs/api/api-reference/chat/create-a-chat-completion)仍列出image输出modalities和image_config。两者不是同一wire。pi 1.0.0的OpenRouter adapter使用chat/completions；当前文档不能证明该旧路径已删除，也不能证明两路径能力完全相同。

文档状态：两页当前可访问；实现状态：官方声明，未live验证。候选选择仍为Q13已确认的pi Chat-wire基线，专用Image API/stream/provider-specific选项另待需求与fixture。不能用新指南的计费承诺补写Chat-wire cancellation保证。新操作的unknown cost保持unknown。

## WEB-02 TypeSafe System One

[官方API参考](https://docs.typesafe.ai/api)确认Bearer鉴权与 `/v1/systemone`、按question ID返回choice/score/noul。choice表示最高概率选项；score表示有序等级的概率加权值；概率与confidence有数值边界。官方接受的state/instructions形状比pi的object state/string instructions更广。候选保持已研究的窄SDK输入，不冒充完整官方API覆盖；更宽结构无需现在泛化。

文档状态：当前有效的供应商声明；实现状态：pi有对应adapter及fixtures，本次未运行，远端兼容未验证。对OpenRouter同协议的证据仍是pi的provider接线和adapter注释，不把TypeSafe官方页视为OpenRouter服务承诺。choice/score校验应有离线反例，供应商声称校准不等于跨模型准确率已证实。

## 实际读取与限制

使用agent-reach网页读取路径：`curl.exe -L --fail --silent --max-time 35 https://r.jina.ai/https://docs.typesafe.ai/api`成功；同一路径读取OpenRouter失败exit1、无内容，随后以web工具直接读取官方页面成功。网页检索的第三方客户端／聚合站只作定位线索，未作为契约权威。没有安装、下载依赖、读取密钥、付费调用或修改治理文件。

实施前仍须以当时官方协议重新核对所选wire；若发生实质不兼容，应重新讨论adapter范围，不能让执行Agent默换为新API并声称完成原验收。此检查不是本次未决产品范围，而是易变外部协议的交付前条件。
