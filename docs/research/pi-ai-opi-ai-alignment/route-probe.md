# O-002 有效目录与实际 route 不一致：隔离离线重现

> 非规范性研究验证；2026-10-03。本文件是观测证据，不是实施准入或目标行为规范。

O-002 的公共库数据流问题已用纯 mock 实际重现。`ProviderRegistry` overlay 改变了 `ProviderCollection::prepare_call` 公开的 `PreparedRoute.wire_api`，但 `ApiMappedProvider` 仍以内部原目录选择 route；两者可以不一致且 attempt 成功 Done。另一个用例中 registry 新增的模型可通过解析、dispatchability 与 prepare，但在 attempt 中被 mapped provider 拒绝为 UnknownModel。

机器可读记录：[route-probe-result.json](route-probe-result.json)。原始静态审计：[O-002 笔记](opi-audit-notes.md#o-002有效模型元数据有两个-authority非启动-route-override-可与实际-wire-脱节)。该笔记写成时尚未运行验证；本文件是其后补充的实测证据。

## 运行边界和基线

- 工作区 HEAD：`f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73`。
- 本次 `git status --short -- crates/opi-ai crates/opi-coding-agent/src/provider_factory.rs` 输出为空；没有修改 runtime/test/config/lock。
- 编译器：`rustc 1.98.0 (88d9e12ae 2026-08-18)`。
- 直接链接已有 `E:/opi/cargo-targets/windows-direct/debug/deps/` 中两份指定 rlib；无 Cargo 构建、依赖安装或下载。
- probe 源码与可执行文件在系统临时目录 `C:/Users/Luiz/AppData/Local/Temp/opi-route-probe-20261003/`；没有在仓库加入 Rust test binary。
- 全部 route 是库自带 `MockProvider`；`CountingAuthResolver` 返回库内固定测试占位串。不访问 HTTP、环境凭据、OS keychain 或模型服务。
- `rustc` 编译 exit **0**；运行 probe exit **0**。断言匹配当前问题，成功退出的含义是“重现得到预期观测”，不是问题已修复。

## 三个实际观测

| 用例 | Registry / PreparedRoute | 实际下层消费 | 终态 |
|---|---|---|---|
| 无 overlay 的控制组 | 都是 `openai-completions` | Chat mock 1 次，Anthropic mock 0 次 | Done，route 一致 |
| 同 ID `m` 的 wire overlay | 都是 `anthropic-messages` | Chat mock 1 次，Anthropic mock 0 次 | Done，route 不一致 |
| 追加 ID `added` | resolve、validate_dispatchable_route、prepare_call 都成功 | 两个 mock 均 0 次；auth resolver 已 1 次 | UnknownModel |

每个用例新建独立 collection；不是前一个用例污染状态。两个 mapped routes 均在 constructor 中通过完整目录验证。第 2 个用例不是“新 wire 没有 adapter”：Anthropic route 确实存在，并有自己的 `aux` 模型，未被调用。

stdout 原文：

```json
{"case":"control","registry_wire":"openai-completions","prepared_wire":"openai-completions","actual_mock_route":"openai-completions","chat_calls":1,"anthropic_calls":0,"auth_calls":1,"terminal":"done","mismatch":false}
{"case":"wire_overlay","registry_wire":"anthropic-messages","prepared_wire":"anthropic-messages","actual_mock_route":"openai-completions","chat_calls":1,"anthropic_calls":0,"auth_calls":1,"terminal":"done","mismatch":true}
{"case":"added_id","registry_resolves":true,"dispatchability_passes":true,"prepare_passes":true,"attempt_result":"UnknownModel","chat_calls":0,"anthropic_calls":0,"auth_calls":1}
```

## 代码证据与外推限制

对应源码：

- `crates/opi-ai/src/registry.rs:176-205` 允许为既有或未来 provider 注册 override；`:230-233` 优先返回 override。
- `crates/opi-ai/src/provider_collection.rs:421-424,538-550` 根据 registry 生成 frozen route facts；`:891-893` dispatch 给 provider 的仍是 request/auth。
- `crates/opi-ai/src/api_mapped.rs:185-200` 根据内部 models 重新查 model 并选择 wire。
- `crates/opi-coding-agent/src/provider_factory.rs:173-193` 在非 active override 上采用 registry overlay 后注册 routes 的同类装配。

本 probe 直接使用这些公开接口，没有执行完整 CLI、extension loader 或模型切换流程；因此“产品非启动 route overlay 也经过该数据流”仍是结合源码的推断，不能伪装成已运行端到端产品用例。未验证 dynamic refresh、endpoint override 或 thinking-map override；它们的相关风险仍须单独验证。

“实际 wire”在本实验准确表示被映射到该 WireApi 的 mock route，使用两个独立 call log 观测，而非网络请求。没有证明生产服务会接受哪个请求，更没有 paid/live 结果。

这里链接的是已有 rlib，SHA256 固定实验材料身份；本 probe 没有通过全新 Cargo 构建核验其全部构建 provenance。静态源代码的数据流与观测一致，相关当前源文件 hash 也记录在结果 JSON。新 Agent 应在准备产品修复时通过正常 repository 测试工具重新建立失败用例。

## 复现命令

将下一节完整源码保存为系统临时目录的 `route_probe.rs`，使用相同编译器和已有 artifact（如 hash 不同，应把结果标为新的实验）。此次实际 PowerShell 命令：

```powershell
rustc --edition=2024 --crate-name opi_route_probe C:/Users/Luiz/AppData/Local/Temp/opi-route-probe-20261003/route_probe.rs --extern opi_ai=E:/opi/cargo-targets/windows-direct/debug/deps/libopi_ai-38f9c4470d925d76.rlib --extern tokio=E:/opi/cargo-targets/windows-direct/debug/deps/libtokio-26851d323d1034af.rlib -L dependency=E:/opi/cargo-targets/windows-direct/debug/deps -o C:/Users/Luiz/AppData/Local/Temp/opi-route-probe-20261003/route_probe.exe
& C:/Users/Luiz/AppData/Local/Temp/opi-route-probe-20261003/route_probe.exe
Get-FileHash -Algorithm SHA256 C:/Users/Luiz/AppData/Local/Temp/opi-route-probe-20261003/route_probe.rs,C:/Users/Luiz/AppData/Local/Temp/opi-route-probe-20261003/route_probe.exe
```

| 材料 | SHA256 |
|---|---|
| route_probe.rs | `52BE335628001C0C1D320D6AA9C32AFF8871D046DBCC2CC4CF5EACACD430F65C` |
| route_probe.exe | `14F8739C157E8374A4CD1ECE17FFC6A761605D11224A01F86574A06F99361548` |
| libopi_ai-38f9c4470d925d76.rlib | `548DFAD9DF214810BF2AFAE50A2DBD11472889FC06A3F004369383D81EAD2E1B` |
| libtokio-26851d323d1034af.rlib | `CF3857DC367B3B80626097519DDC1E274841E9C07C0C748A262E9766D50ADC67` |

Windows executable 可能受链接时间等影响；跨次复现应主要断言 JSON 语义与 route call logs，不能要求重新链接后二进制 hash 恒等。

## 完整隔离 probe 源码

以下代码仅用于捕获当前问题；它的断言不应复制为把不一致永久固定下来的回归测试。目标 regression test 应在目标契约获确认后断言同一 binding 或 preflight 显式拒绝。

```rust
use std::sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}};

use opi_ai::{ApiMappedProvider, AuthProvenanceSource, CompatMetadata, ModelCapabilities,
    ModelInfo, Provider, ProviderCollection, ProviderRegistry, WireApi};
use opi_ai::provider::{CacheRetention, ProviderError, Request, ThinkingConfig};
use opi_ai::provider_collection::{CompletedRequest, drain_to_completion};
use opi_ai::test_support::{CountingAuthResolver, MockProvider, text_response};

type Calls = Arc<Mutex<Vec<Request>>>;

fn model(id: &str, wire: WireApi) -> ModelInfo {
    ModelInfo::new(id, id, wire, ModelCapabilities::new(100_000, 1_000).with_streaming(true))
}

fn setup(overlay: Option<ModelInfo>) -> (ProviderCollection, Calls, Calls, Arc<AtomicU32>) {
    let chat_model = model("m", WireApi::OpenAiCompletions);
    let anthropic_model = model("aux", WireApi::AnthropicMessages);
    let chat = MockProvider::new_with_models("acme", vec![chat_model.clone()], vec![text_response("chat")]);
    let anthropic = MockProvider::new_with_models("acme", vec![anthropic_model.clone()], vec![text_response("anthropic")]);
    let chat_calls = chat.call_log_handle();
    let anthropic_calls = anthropic.call_log_handle();
    let mapped = ApiMappedProvider::try_new(
        "acme", vec![chat_model, anthropic_model],
        [
            (WireApi::OpenAiCompletions, Box::new(chat) as Box<dyn Provider>),
            (WireApi::AnthropicMessages, Box::new(anthropic) as Box<dyn Provider>),
        ],
    ).unwrap();
    let mut registry = ProviderRegistry::new();
    if let Some(overlay) = overlay {
        registry.register_model("acme", overlay).unwrap();
    }
    let mut collection = ProviderCollection::from_registry(registry);
    let auth_calls = Arc::new(AtomicU32::new(0));
    collection.register_route(
        Box::new(mapped), Arc::new(CountingAuthResolver::new(Arc::clone(&auth_calls))),
        AuthProvenanceSource::Static, CompatMetadata::default(),
    ).unwrap();
    (collection, chat_calls, anthropic_calls, auth_calls)
}

fn request(id: &str) -> Request {
    Request {
        model: format!("acme:{id}"), system: None, messages: vec![], tools: vec![],
        max_tokens: Some(100), temperature: None, thinking: ThinkingConfig::default(),
        stop_sequences: vec![], metadata: None, cancel: Default::default(), timeout: None,
        extra_headers: vec![], cache_retention: CacheRetention::None, session_id: None,
    }
}

async fn run_case(name: &str, id: &str, overlay: Option<ModelInfo>, expect_mismatch: bool) {
    let (collection, chat_calls, anthropic_calls, auth_calls) = setup(overlay);
    let spec = format!("acme:{id}");
    let resolved_wire = collection.resolve(&spec).unwrap().1.wire_api;
    let prepared = collection.prepare_call(&spec, request(id)).await.unwrap();
    let prepared_wire = prepared.route().wire_api;
    assert_eq!(resolved_wire, prepared_wire);
    let result = drain_to_completion(prepared.start_attempt().unwrap()).await;
    assert!(matches!(result, Ok(CompletedRequest::Done { .. })));
    let chat_count = chat_calls.lock().unwrap().len();
    let anthropic_count = anthropic_calls.lock().unwrap().len();
    assert_eq!((chat_count, anthropic_count), (1, 0));
    assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
    assert_eq!(prepared_wire != WireApi::OpenAiCompletions, expect_mismatch);
    println!("{{\"case\":\"{name}\",\"registry_wire\":\"{resolved_wire}\",\"prepared_wire\":\"{prepared_wire}\",\"actual_mock_route\":\"openai-completions\",\"chat_calls\":{chat_count},\"anthropic_calls\":{anthropic_count},\"auth_calls\":1,\"terminal\":\"done\",\"mismatch\":{expect_mismatch}}}");
}

async fn run_added_case() {
    let (collection, chat_calls, anthropic_calls, auth_calls) = setup(Some(model("added", WireApi::OpenAiCompletions)));
    assert_eq!(collection.resolve("acme:added").unwrap().1.id, "added");
    collection.validate_dispatchable_route("acme:added").unwrap();
    let prepared = collection.prepare_call("acme:added", request("added")).await.unwrap();
    let result = drain_to_completion(prepared.start_attempt().unwrap()).await;
    assert!(matches!(result, Err(ProviderError::UnknownModel { ref provider_id, ref model_id })
        if provider_id == "acme" && model_id == "added"));
    assert_eq!(chat_calls.lock().unwrap().len(), 0);
    assert_eq!(anthropic_calls.lock().unwrap().len(), 0);
    assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
    println!("{{\"case\":\"added_id\",\"registry_resolves\":true,\"dispatchability_passes\":true,\"prepare_passes\":true,\"attempt_result\":\"UnknownModel\",\"chat_calls\":0,\"anthropic_calls\":0,\"auth_calls\":1}}");
}

fn main() {
    tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
        run_case("control", "m", None, false).await;
        run_case("wire_overlay", "m", Some(model("m", WireApi::AnthropicMessages)), true).await;
        run_added_case().await;
    });
}
```

本次仓库测试影响：**none**。临时 probe 为实际新增并运行的隔离研究验证，未作为仓库测试交付。没有修改此前审计笔记，以保留原观测时间和静态/实测证据层次。
