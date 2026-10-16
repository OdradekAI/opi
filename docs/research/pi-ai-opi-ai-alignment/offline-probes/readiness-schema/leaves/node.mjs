import {readFileSync} from 'node:fs';
for (const c of JSON.parse(readFileSync(process.argv[2],'utf8'))) {
  console.log(JSON.stringify({id:c.id,result:c.kind==='number-string'?String(Number(c.value)):URL.canParse(c.value)}));
}
