import { Button, Card } from "@heroui/react";
import { BookOpenText, ChevronRight } from "lucide-react";
export function resourceIdentity(id: string, kind: string, author?: string | null) {
  const identities: Record<string, {title: string; type: string; monogram: string; cover: string}> = {
    "matthew-henry": {title:"Matthew Henry", type:"Commentary", monogram:"MH", cover:"henry"},
    "abbott-smith": {title:"Abbott-Smith’s Greek Lexicon", type:"Greek lexicon · 1922", monogram:"AS", cover:"abbott"},
    "easton": {title:"Easton’s Bible Dictionary", type:"Bible dictionary · 1897", monogram:"E", cover:"easton"},
    "nave": {title:"Nave’s Topical Bible", type:"Topical Bible", monogram:"N", cover:"nave"},
    "tsk": {title:"Treasury of Scripture Knowledge", type:"Cross-references", monogram:"TSK", cover:"tsk"},
    "strongs-hebrew": {title:"Strong’s Hebrew Dictionary", type:"Hebrew dictionary", monogram:"א", cover:"strongs"},
    "strongs-greek": {title:"Strong’s Greek Dictionary", type:"Greek dictionary", monogram:"α", cover:"greek"},
    "robinson-tr": {title:"Robinson’s Textus Receptus", type:"Greek source text", monogram:"TR", cover:"tr"},
  };
  return identities[id] || {title: author || id, type:kind.replaceAll("_", " "), monogram: id.slice(0,3).toUpperCase(), cover:"default"};
}
export function LibraryResourceCard({title, type, description, monogram, cover, disabled, onOpen}: {
  title:string; type:string; description:string; monogram:string; cover:string; disabled?:boolean; onOpen:()=>void;
}) {
  return <Card className="resource-card"><Card.Content>
    <div className={`resource-cover resource-${cover}`} aria-hidden="true"><span>{monogram}</span><BookOpenText size={28}/></div>
    <div><div className="eyebrow">{type}</div><h2>{title}</h2><p className="muted small">{description}</p>
      <Button size="sm" variant="secondary" isDisabled={disabled} onPress={onOpen}>{disabled ? "Unavailable in preview" : "Open resource"}<ChevronRight size={14}/></Button>
    </div>
  </Card.Content></Card>;
}
