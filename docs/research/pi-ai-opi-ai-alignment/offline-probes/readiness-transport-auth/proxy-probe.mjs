import path from 'node:path';
import { pathToFileURL } from 'node:url';
const root = process.argv[2];
const {resolveHttpProxyUrlForTarget} = await import(pathToFileURL(path.join(root,'packages/ai/src/utils/node-http-proxy.ts')));
for(const key of Object.keys(process.env)) if(/proxy/i.test(key)) delete process.env[key];
const cases=[
  {id:'scoped-lower',env:{https_proxy:'http://scoped-lower:80',HTTPS_PROXY:'http://scoped-upper:80'},ambient:{https_proxy:'http://ambient-lower:80'},url:'https://target.example'},
  {id:'scoped-upper-empty-lower',env:{https_proxy:'',HTTPS_PROXY:'http://scoped-upper:80'},ambient:{https_proxy:'http://ambient-lower:80'},url:'https://target.example'},
  {id:'ambient-lower',env:{},ambient:{https_proxy:'http://ambient-lower:80',HTTPS_PROXY:'http://ambient-upper:80'},url:'https://target.example'},
  {id:'protocol-over-all',env:{https_proxy:'http://protocol:80',all_proxy:'http://all:80'},url:'https://target.example'},
  {id:'all-fallback',env:{all_proxy:'http://all:80'},url:'https://target.example'},
  {id:'no-proxy-domain',env:{https_proxy:'http://proxy:80',no_proxy:'.example'},url:'https://a.example'},
  {id:'no-proxy-port-match',env:{https_proxy:'http://proxy:80',no_proxy:'target.example:443'},url:'https://target.example'},
  {id:'no-proxy-port-miss',env:{https_proxy:'http://proxy:80',no_proxy:'target.example:444'},url:'https://target.example'},
  {id:'no-proxy-ipv6',env:{http_proxy:'http://proxy:80',no_proxy:'[::1]:8080'},url:'http://[::1]:8080'},
  {id:'no-proxy-wildcard',env:{https_proxy:'http://proxy:80',no_proxy:'*'},url:'https://target.example'},
  {id:'domain-boundary',env:{https_proxy:'http://proxy:80',no_proxy:'.example'},url:'https://notexample'},
  {id:'unsupported-socks',env:{https_proxy:'socks5://proxy:1080'},url:'https://target.example'},
  {id:'invalid-proxy',env:{https_proxy:'http://['},url:'https://target.example'},
  {id:'bypass-invalid-before-parse',env:{https_proxy:'http://[',no_proxy:'*'},url:'https://target.example'},
  {id:'ws-explicit-http',env:{ws_proxy:'http://proxy:80'},url:'ws://target.example'},
  {id:'ws-bare-prefix',env:{ws_proxy:'proxy:80'},url:'ws://target.example'},
];
for(const c of cases){
  for(const key of Object.keys(process.env)) if(/proxy/i.test(key)) delete process.env[key];
  Object.assign(process.env,c.ambient??{});
  let result;try{result={ok:true,proxy:resolveHttpProxyUrlForTarget(c.url,c.env)?.href??null}}catch(error){result={ok:false,name:error.name,message:error.message}}
  console.log(JSON.stringify({...c,result}));
}
