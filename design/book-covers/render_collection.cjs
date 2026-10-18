const fs=require('node:fs'),path=require('node:path');
const {Resvg}=require(process.env.FONTES_RESVG_MODULE||'/tmp/fontes-svg-render/node_modules/@resvg/resvg-js');
const root=__dirname,books=JSON.parse(fs.readFileSync(path.join(root,'collection/books.json'))).books;
fs.mkdirSync(path.join(root,'collection/contact-sheets'),{recursive:true});
const escape=s=>s.replaceAll('&','&amp;').replaceAll('<','&lt;');
for(let batch=0;batch<4;batch++){
 const selected=books.slice(batch*18,batch*18+18),rows=Math.ceil(selected.length/3);
 let board=`<svg xmlns="http://www.w3.org/2000/svg" width="900" height="${rows*352+56}" viewBox="0 0 900 ${rows*352+56}"><path fill="#eae6dd" d="M0 0h900v${rows*352+56}H0z"/><text x="20" y="30" font-family="IBM Plex Sans" font-size="17" fill="#253D43">Covenant Forms · ${selected[0].name} — ${selected.at(-1).name}</text>`;
 selected.forEach((b,n)=>{
  let svg=fs.readFileSync(path.resolve(root,'collection',b.art),'utf8').replaceAll('id="arch"',`id="arch${b.id}"`).replaceAll('url(#arch)',`url(#arch${b.id})`);
  const inner=svg.replace(/<svg[^>]*>|<\/svg>/g,'');
  const x=20+(n%3)*300,y=50+Math.floor(n/3)*352;
  board+=`<svg x="${x}" y="${y}" width="260" height="278" viewBox="0 0 600 640">${inner}</svg><text x="${x}" y="${y+300}" font-family="IBM Plex Sans" font-size="16" fill="#253D43">${b.id}. ${escape(b.name)}</text><text x="${x}" y="${y+324}" font-family="IBM Plex Sans" font-size="12" fill="#80665e">${escape(b.title)}</text>`;
 });
 board+='</svg>';
 fs.writeFileSync(path.join(root,`collection/contact-sheets/sheet-${batch+1}.png`),new Resvg(board,{font:{fontFiles:[path.resolve(root,'../../assets/ui/fonts/IBMPlexSans-Regular.ttf')],loadSystemFonts:false}}).render().asPng());
}
console.log('Rendered 66 SVG covers in four contact sheets.');
