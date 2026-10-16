
const screens=await Promise.all(['3:47','3:48','3:49','3:50','3:51','3:52','3:53','3:54'].map(id=>figma.getNodeByIdAsync(id)));
const routes={'Nav/Read':'3:47','Open passage picker':'3:48','Verse/16':'3:49','Nav/My notes':'3:50','Add note':'3:50','Highlight selection':'3:50','Nav/Search':'3:51','Ask your library':'3:51','Nav/Genealogies':'3:52','Book overview':'3:53','Nav/Preferences':'3:54','Reading preferences':'3:54','Choose translation':'3:54','Close panel':'3:47','Close picker':'3:47','Open selected passage':'3:47','Return to reading':'3:47','Cancel note':'3:49','Save note':'3:47','Resource/Strong’s Concordance':'3:49','Occurrence/John 3:16':'3:47','Citation/John 3:16':'3:47'};
const affected=[];
for(const root of screens){for(const n of root.findAll(()=>true)){const destination=routes[n.name];if(destination&&'setReactionsAsync'in n){await n.setReactionsAsync([{trigger:{type:'ON_CLICK'},actions:[{type:'NODE',destinationId:destination,navigation:'NAVIGATE',transition:{type:'DISSOLVE',easing:{type:'EASE_OUT'},duration:0.18}}]}]);affected.push(n.id);}}}
figma.currentPage.flowStartingPoints=[{nodeId:'3:47',name:'Fontes · Begin reading'}];
return {createdNodeIds:[],mutatedNodeIds:[...affected,figma.currentPage.id],interactionCount:affected.length};

