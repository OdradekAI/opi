#[cfg(test)]
mod tests {
 use bytes::BytesMut;
 use ratchet_core::{Message,Role,WebSocket,WebSocketConfig};
 use ratchet_deflate::{Deflate,DeflateConfig,DeflateExtProvider};
 use ratchet_ext::{Extension,ExtensionProvider,ExtensionEncoder,ExtensionDecoder,FrameHeader,RsvBits,OpCode};
 use std::{pin::Pin,task::{Context,Poll},io::Cursor};
 use tokio::io::{AsyncRead,AsyncWrite,ReadBuf};
 #[derive(Debug)] struct PmdAdapter{inner:Deflate,current:bool}
 impl Extension for PmdAdapter{fn bits(&self)->RsvBits{self.inner.bits()}}
 impl ExtensionEncoder for PmdAdapter{type Error=ratchet_deflate::DeflateExtensionError;fn encode(&mut self,p:&mut BytesMut,h:&mut FrameHeader)->Result<(),Self::Error>{self.inner.encode(p,h)}}
 impl ExtensionDecoder for PmdAdapter{type Error=ratchet_deflate::DeflateExtensionError;fn decode(&mut self,p:&mut BytesMut,h:&mut FrameHeader)->Result<(),Self::Error>{if matches!(h.opcode,OpCode::Text|OpCode::Binary){self.current=h.rsv1}let r=if self.current{self.inner.decode(p,h)}else{Ok(())};if h.fin{self.current=false}r}}
 struct MemoryStream{input:Cursor<Vec<u8>>,output:Vec<u8>}
 impl AsyncRead for MemoryStream{fn poll_read(mut self:Pin<&mut Self>,cx:&mut Context<'_>,buf:&mut ReadBuf<'_>)->Poll<std::io::Result<()>>{Pin::new(&mut self.input).poll_read(cx,buf)}}
 impl AsyncWrite for MemoryStream{fn poll_write(mut self:Pin<&mut Self>,_:&mut Context<'_>,buf:&[u8])->Poll<std::io::Result<usize>>{self.output.extend_from_slice(buf);Poll::Ready(Ok(buf.len()))}fn poll_flush(self:Pin<&mut Self>,_:&mut Context<'_>)->Poll<std::io::Result<()>>{Poll::Ready(Ok(()))}fn poll_shutdown(self:Pin<&mut Self>,_:&mut Context<'_>)->Poll<std::io::Result<()>>{Poll::Ready(Ok(()))}}
 fn hex(s:&str)->Vec<u8>{s.as_bytes().chunks_exact(2).map(|b|u8::from_str_radix(std::str::from_utf8(b).unwrap(),16).unwrap()).collect()}
 #[tokio::test] async fn readiness_memory_public_codec_pmd_and_prior_regressions(){
 let fixtures:Vec<serde_json::Value>=serde_json::from_str(include_str!("../memory-codec-cases.json")).unwrap();assert_eq!(fixtures.len(),12);
 for f in fixtures {let name=f["case"].as_str().unwrap();let mut h=http::HeaderMap::new();if let Some(ext)=f["extension"].as_str(){h.insert("sec-websocket-extensions",http::HeaderValue::from_str(ext).unwrap());}
 let provider=DeflateExtProvider::with_config(DeflateConfig{request_server_no_context_takeover:false,request_client_no_context_takeover:false,..Default::default()});let ext=provider.negotiate_client(&h).unwrap().map(|inner|PmdAdapter{inner,current:false});
 let stream=MemoryStream{input:Cursor::new(hex(f["hex"].as_str().unwrap())),output:Vec::new()};let mut ws=WebSocket::from_upgraded(WebSocketConfig{max_message_size:usize::MAX},stream,ext,BytesMut::new(),Role::Client);let mut buf=BytesMut::new();let mut messages=Vec::new();let mut protocol_error=false;
 for _ in 0..20 {match ws.read(&mut buf).await{Ok(Message::Text)=>{messages.push(String::from_utf8(buf.to_vec()).unwrap());buf.clear()},Ok(Message::Close(_))=>break,Ok(Message::Ping(_)|Message::Pong(_))=>{},Ok(_)=>panic!("unexpected message: {name}"),Err(error)=>{assert!(error.is_protocol(),"{name}: {error:?}");protocol_error=true;break}}}
 assert_eq!(protocol_error,f["expectProtocolError"].as_bool().unwrap(),"{name}");let expected:Vec<_>=f["expectedMessages"].as_array().unwrap().iter().map(|v|v.as_str().unwrap().to_owned()).collect();assert_eq!(messages,expected,"{name}");
 }
 }
}
