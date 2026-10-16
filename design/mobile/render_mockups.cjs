const fs = require('node:fs');
const path = require('node:path');
const {Resvg} = require('/tmp/fontes-svg-render/node_modules/@resvg/resvg-js');
const fontFiles=fs.readdirSync(path.join(__dirname,'assets/fonts')).filter(n=>n.endsWith('.ttf')).map(n=>path.join(__dirname,'assets/fonts',n));
function render(input,output,scale){
  const r=new Resvg(fs.readFileSync(input),{font:{fontFiles,loadSystemFonts:false,defaultFontFamily:'IBM Plex Sans'},fitTo:{mode:'zoom',value:scale}});
  fs.writeFileSync(output,r.render().asPng());
}
(async()=>{
  for(const name of fs.readdirSync(path.join(__dirname,'screens')).filter(n=>n.endsWith('.svg'))){
    const input=path.join(__dirname,'screens',name);
    render(input,input.replace('.svg','.png'),2);
  }
  render(path.join(__dirname,'mobile-mockups.svg'),path.join(__dirname,'mobile-mockups.png'),1);
  console.log('Rendered 8 screens at 2× and overview at 1×');
})();
