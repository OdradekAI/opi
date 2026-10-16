"""Offline RFC 1951 bitstream distance audit; not a product decoder."""
class Bits:
 def __init__(self,b):self.b=b;self.p=0
 def get(self,n):
  if self.p+n>len(self.b)*8:raise EOFError()
  v=sum(((self.b[(self.p+i)//8]>>((self.p+i)%8))&1)<<i for i in range(n));self.p+=n;return v
 def align(self):self.p=(self.p+7)//8*8
class Huff:
 def __init__(self,lens):
  count=[0]*16
  for x in lens:
   if x:count[x]+=1
  code=0;next=[0]*16
  for i in range(1,16):code=(code+count[i-1])*2;next[i]=code
  self.tab={}
  for sym,n in enumerate(lens):
   if n:
    c=next[n];next[n]+=1;r=int(f'{c:0{n}b}'[::-1],2);self.tab[n,r]=sym
 def get(self,b):
  c=0
  for n in range(1,16):
   c|=b.get(1)<<(n-1)
   if (n,c) in self.tab:return self.tab[n,c]
  raise ValueError('invalid huffman')
LBASE=[3,4,5,6,7,8,9,10,11,13,15,17,19,23,27,31,35,43,51,59,67,83,99,115,131,163,195,227,258]
LEXTRA=[0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0]
DBASE=[1,2,3,4,5,7,9,13,17,25,33,49,65,97,129,193,257,385,513,769,1025,1537,2049,3073,4097,6145,8193,12289,16385,24577]
DEXTRA=[0,0,0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10,11,11,12,12,13,13]
def audit(data):
 b=Bits(data);maxdist=0;size=0;blocks=[]
 while b.p+3<=len(data)*8:
  final=b.get(1);typ=b.get(2);blocks.append(typ)
  if typ==0:
   b.align();n=b.get(16);assert b.get(16)==(n^65535)
   for _ in range(n):b.get(8)
   size+=n
  else:
   if typ==1:lit=Huff([8]*144+[9]*112+[7]*24+[8]*8);dist=Huff([5]*32)
   elif typ==2:
    nl=b.get(5)+257;nd=b.get(5)+1;nc=b.get(4)+4;cl=[0]*19
    order=[16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15]
    for i in range(nc):cl[order[i]]=b.get(3)
    table=Huff(cl);lengths=[]
    while len(lengths)<nl+nd:
     x=table.get(b)
     if x<16:lengths.append(x)
     elif x==16:lengths.extend([lengths[-1]]*(b.get(2)+3))
     elif x==17:lengths.extend([0]*(b.get(3)+3))
     else:lengths.extend([0]*(b.get(7)+11))
    assert len(lengths)==nl+nd;lit=Huff(lengths[:nl]);dist=Huff(lengths[nl:])
   else:raise ValueError('reserved block')
   while True:
    x=lit.get(b)
    if x<256:size+=1
    elif x==256:break
    else:
     i=x-257;length=LBASE[i]+b.get(LEXTRA[i]);d=dist.get(b);distance=DBASE[d]+b.get(DEXTRA[d]);maxdist=max(maxdist,distance);size+=length
  if final:break
 return dict(maxDistance=maxdist,decodedBytes=size,blocks=blocks)
if __name__=='__main__':
 import pathlib,json,zlib
 p=pathlib.Path(__file__).parent;rows=json.loads((p/'results.json').read_text())['results'];results=[]
 for r in rows:
  if r['case']['id']!='window8':continue
  data=bytes.fromhex(r['wire']['clientFramesHex']);first=None
  while len(data)>2:
   a,b=data[:2];n=b&127;off=2
   if n==126:n=int.from_bytes(data[off:off+2],'big');off+=2
   elif n==127:n=int.from_bytes(data[off:off+8],'big');off+=8
   mask=data[off:off+4] if b&128 else b'';off+=len(mask);payload=data[off:off+n];data=data[off+n:]
   if mask:payload=bytes(x^mask[i%4] for i,x in enumerate(payload))
   if a&15==1:first=(bool(a&64),payload);break
  inbound=[]
  for x in r['wire']['payloads']:
   v=audit(bytes.fromhex(x['compressedHex'])+b'\0\0\xff\xff');assert v['maxDistance']<=256;inbound.append(v)
  row=dict(runtime=r['runtime'],inbound=inbound,clientDataFrame=first is not None)
  if first:
   compressed,payload=first;row['compressedOutbound']=compressed
   if compressed:
    v=audit(payload+b'\0\0\xff\xff');assert v['maxDistance']<=256;assert zlib.decompressobj(-8).decompress(payload+b'\0\0\xff\xff').decode()==(p/'outbound.txt').read_text();row['outbound']=v;row['strict8DecoderMatches']=True
   else:assert payload.decode()==(p/'outbound.txt').read_text();row['uncompressedMatches']=True
  results.append(row)
 (p/'window8-audit.json').write_text(json.dumps(dict(method='independent RFC1951 distance parse plus raw -8 zlib decoder; first compressed client message',results=results),indent=2)+'\n');print(json.dumps(results,indent=2))
