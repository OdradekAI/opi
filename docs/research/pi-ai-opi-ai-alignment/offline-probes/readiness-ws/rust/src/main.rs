use bytes::BytesMut;
use ratchet_core::{subscribe_with,Message,SubprotocolRegistry,WebSocketConfig,WebSocket,Role};
use ratchet_deflate::{DeflateExtProvider,DeflateConfig,Deflate};
use ratchet_ext::{ExtensionProvider,Extension,ExtensionEncoder,ExtensionDecoder,FrameHeader,RsvBits,OpCode};
use tokio::{net::TcpStream,io::{AsyncReadExt,AsyncWriteExt}};
use sha1::{Sha1,Digest};use base64::Engine;
fn provider()->DeflateExtProvider{DeflateExtProvider::with_config(DeflateConfig{request_server_no_context_takeover:false,request_client_no_context_takeover:false,..Default::default()})}
#[derive(Debug)]
struct PmdAdapter{inner:Deflate,current_compressed:bool,guard:bool}
impl Extension for PmdAdapter{fn bits(&self)->RsvBits{self.inner.bits()}}
impl ExtensionEncoder for PmdAdapter{type Error=ratchet_deflate::DeflateExtensionError;fn encode(&mut self,p:&mut BytesMut,h:&mut FrameHeader)->Result<(),Self::Error>{self.inner.encode(p,h)}}
impl ExtensionDecoder for PmdAdapter{type Error=ratchet_deflate::DeflateExtensionError;fn decode(&mut self,p:&mut BytesMut,h:&mut FrameHeader)->Result<(),Self::Error>{
 if !self.guard{return self.inner.decode(p,h)}
 if matches!(h.opcode,OpCode::Text|OpCode::Binary){self.current_compressed=h.rsv1;}
 let result=if self.current_compressed{self.inner.decode(p,h)}else{Ok(())};
 if h.fin{self.current_compressed=false;}result
}}
struct PmdProvider;
impl ExtensionProvider for PmdProvider{type Extension=PmdAdapter;type Error=ratchet_deflate::DeflateExtensionError;
 fn apply_headers(&self,h:&mut http::HeaderMap){provider().apply_headers(h)}
 fn negotiate_client(&self,h:&http::HeaderMap)->Result<Option<PmdAdapter>,Self::Error>{provider().negotiate_client(h).map(|v|v.map(|inner|PmdAdapter{inner,current_compressed:false,guard:false}))}
 fn negotiate_server(&self,h:&http::HeaderMap)->Result<Option<(PmdAdapter,http::HeaderValue)>,Self::Error>{provider().negotiate_server(h).map(|v|v.map(|(inner,h)|(PmdAdapter{inner,current_compressed:false,guard:false},h)))}
}
async fn owned(mut stream:TcpStream,url:&str)->Result<WebSocket<TcpStream,PmdAdapter>,String>{
 let uri:http::Uri=url.parse().map_err(|e|format!("uri:{e}"))?;
 let nonce:[u8;16]=rand::random();let key=base64::engine::general_purpose::STANDARD.encode(nonce);
 let request=format!("GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Extensions: permessage-deflate; client_max_window_bits\r\nAuthorization: Bearer offline-fake-token\r\nX-Opi-Offline-Canary: pi130-canary\r\n\r\n",uri.path_and_query().map(|v|v.as_str()).unwrap_or("/"),uri.authority().unwrap());
 stream.write_all(request.as_bytes()).await.map_err(|e|e.to_string())?;
 let mut data=Vec::new();let end=loop{let mut b=[0u8;8192];let n=stream.read(&mut b).await.map_err(|e|e.to_string())?;if n==0{return Err("eof-head".into())} data.extend_from_slice(&b[..n]);if let Some(p)=data.windows(4).position(|v|v==b"\r\n\r\n"){break p+4}};
 let count=data[..end].iter().filter(|b|**b==b'\n').count();let mut slots=vec![httparse::EMPTY_HEADER;count];let mut response=httparse::Response::new(&mut slots);response.parse(&data[..end]).map_err(|e|e.to_string())?;
 if response.version!=Some(1)||response.code!=Some(101){return Err("status".into())}
 let values=|name:&str|response.headers.iter().filter(|h|h.name.eq_ignore_ascii_case(name)).map(|h|h.value).collect::<Vec<_>>();
 let exactly=|name:&str|{let v=values(name);if v.len()==1{Ok(v[0])}else{Err(format!("non-singleton:{name}"))}};
 let token=|v:&[u8],expected:&str|std::str::from_utf8(v).unwrap_or("").split(',').any(|s|s.trim().eq_ignore_ascii_case(expected));
 if !token(exactly("upgrade")?,"websocket")||!token(exactly("connection")?,"upgrade"){return Err("upgrade-token".into())}
 let want=base64::engine::general_purpose::STANDARD.encode(Sha1::digest(format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11")));
 if exactly("sec-websocket-accept")?!=want.as_bytes(){return Err("accept".into())}
 if !values("sec-websocket-protocol").is_empty(){return Err("unsolicited-subprotocol".into())}
 let mut selected=http::HeaderMap::new();let extensions=values("sec-websocket-extensions");
 if extensions.len()>1{return Err("duplicate-extension-header".into())}
 if let Some(value)=extensions.first(){let s=std::str::from_utf8(value).map_err(|e|e.to_string())?;if s.contains(',')||s.split(';').next().unwrap_or("").trim()!="permessage-deflate"{return Err("unsolicited-extension".into())}selected.insert("sec-websocket-extensions",http::HeaderValue::from_bytes(value).map_err(|e|e.to_string())?);}
 let ext=provider().negotiate_client(&selected).map_err(|e|format!("pmd:{e:?}"))?;if !extensions.is_empty()&&ext.is_none(){return Err("invalid-pmd-negotiation".into())}
 Ok(WebSocket::from_upgraded(WebSocketConfig{max_message_size:usize::MAX},stream,ext.map(|inner|PmdAdapter{inner,current_compressed:false,guard:true}),BytesMut::from(&data[end..]),Role::Client))
}
#[tokio::main(flavor="current_thread")]
async fn main(){
 let args:Vec<_>=std::env::args().collect();let url=&args[1];let mode=args.get(2).map(String::as_str).unwrap_or("native");let uri:http::Uri=url.parse().unwrap();
 let stream=TcpStream::connect(format!("{}:{}",uri.host().unwrap(),uri.port_u16().unwrap())).await.unwrap();
 let result=if mode=="owned"{owned(stream,url).await}else{let request=http::Request::get(url).header("authorization","Bearer offline-fake-token").header("x-opi-offline-canary","pi130-canary").body(()).unwrap(); subscribe_with(WebSocketConfig{max_message_size:usize::MAX},stream,request,&PmdProvider,SubprotocolRegistry::default()).await.map(|v|v.websocket).map_err(|e|format!("{e:?}"))};
 let mut messages=Vec::new();let mut controls=Vec::new();
 match result{Ok(mut ws)=>{let mut outbound=BytesMut::from(include_str!("../../outbound.txt").as_bytes());let _=ws.write(&mut outbound,ratchet_core::PayloadType::Text).await;let _=ws.flush().await;let mut buf=BytesMut::new();let mut error=None;
 for _ in 0..20 {match tokio::time::timeout(std::time::Duration::from_millis(500),ws.read(&mut buf)).await{
 Ok(Ok(Message::Text))=>{messages.push(String::from_utf8_lossy(&buf).into_owned());buf.clear();},Ok(Ok(Message::Ping(_)))=>{controls.push("ping");},Ok(Ok(Message::Pong(_)))=>{controls.push("pong");},Ok(Ok(Message::Close(_)))=>break,Ok(Ok(_))=>buf.clear(),Ok(Err(e))=>{error=Some(format!("{e:?}"));break},Err(_)=>break}}
 println!("{}",serde_json::json!({"opened":true,"messages":messages,"controls":controls,"error":error}));},Err(e)=>println!("{}",serde_json::json!({"opened":false,"messages":messages,"controls":controls,"error":e}))}
}
