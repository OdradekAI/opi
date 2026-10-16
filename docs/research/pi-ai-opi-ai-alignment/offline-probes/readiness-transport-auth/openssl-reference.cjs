// The exact installed Pi dependency path; no package manager or auto-resolution.
const fs=require('node:fs');
const path=require('node:path');
const crypto=require('node:crypto');
const {createRequire}=require('node:module');
const root=createRequire(path.join(process.argv[2],'package.json'));
const getJwsSign=root('google-auth-library/build/src/gtoken/jwsSign.js').getJwsSign;
const jws=root('jws');
const key=fs.readFileSync(process.argv[3],'utf8');
try {
 const token=getJwsSign({iss:'synthetic@example.invalid',scope:'synthetic-scope',key,additionalClaims:{iat:1,exp:3601}});
 const parts=token.split('.');const signature=Buffer.from(parts[2],'base64url');
 console.log(JSON.stringify({accepted:true,secured_input:parts.slice(0,2).join('.'),signature_sha256:crypto.createHash('sha256').update(signature).digest('hex'),signature_bytes:signature.length,verified:jws.verify(token,'RS256',crypto.createPublicKey(key))}));
}catch(error){console.log(JSON.stringify({accepted:false,name:error.name,message:error.message}));}
