const tls=require('node:tls');
const crypto=require('node:crypto');
const digest=x=>crypto.createHash('sha256').update(new crypto.X509Certificate(x).raw).digest('hex');
console.log(JSON.stringify({node:process.version,bundled_count:tls.rootCertificates.length,
 default_count:tls.getCACertificates('default').length,
 bundled_sha256:tls.rootCertificates.map(digest),default_sha256:tls.getCACertificates('default').map(digest)},null,2));
