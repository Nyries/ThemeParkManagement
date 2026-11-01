import type { ToolState } from "@/types/park/tool";
import { InspectorPanel } from "../shell/InspectorPanel";
import { Button } from "../ui/button";
import {
  ChevronDown,
  CircleChevronDown,
  CircleChevronUp,
  CirclePlus,
  Folder,
} from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../ui/tooltip";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogOverlay,
  DialogPortal,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Form, FormField, FormInput, FormLabel } from "../ui/form";
import { useState } from "react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";

interface EditorProps {
  tool: ToolState;
  onToolChange: (tool: ToolState) => void;
}

const ARCHETYPES = [
  { value: "plain", label: "Plaine (Plain)" },
  { value: "city", label: "Ville (City)" },
  { value: "mountain", label: "Montagne (Mountain)" },
  { value: "forest", label: "Forêt (Forest)" },
];

const DEFAULT_TERRAINS = [
  { value: "grass", label: "Herbe (Grass)" },
  { value: "dirt", label: "Terre (Dirt)" },
  { value: "stone", label: "Pierre (Stone)" },
  { value: "water", label: "Eau (Water)" },
];

export function Editor({ tool, onToolChange }: EditorProps) {
  return (
    <div className="flex h-full w-full">
      <div className="flex flex-1 min-w-0 justify-center items-center gap-4">
        <TooltipProvider>
          <Dialog>
            <DialogTrigger asChild>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="secondary"
                    className="flex-1 h-1/2 justify-center max-w-sm flex-col gap-4 text-lg rounded-lg"
                  >
                    <Folder className="size-50"></Folder>
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom">
                  Modifier une map existante
                </TooltipContent>
              </Tooltip>
            </DialogTrigger>
            <DialogPortal></DialogPortal>
          </Dialog>
          <Dialog>
            <Tooltip>
              <TooltipTrigger asChild>
                <DialogTrigger asChild>
                  <Button
                    variant="secondary"
                    className="flex-1 h-1/2 justify-center max-w-sm flex-col gap-4 text-lg rounded-lg"
                  >
                    <CirclePlus className="size-50"></CirclePlus>
                  </Button>
                </DialogTrigger>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                Créer une nouvelle map
              </TooltipContent>
            </Tooltip>
            <CreatingMapDialog></CreatingMapDialog>
          </Dialog>
        </TooltipProvider>
      </div>
      <InspectorPanel
        selection={null}
        tool={tool}
        onToolChange={onToolChange}
      ></InspectorPanel>
    </div>
  );
}

function CreatingMapDialog() {
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [formData, setFormData] = useState({
    name: "",
    archetype: ARCHETYPES[0].value,
    defaultTerrain: DEFAULT_TERRAINS[0].value,
    width: 10,
    height: 10,
    visitor_spawning_rate: 100,
    max_visitors: 100,
  });

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value, type } = e.target;

    setFormData((prev) => ({
      ...prev,
      [name]: type === "number" ? (value === "" ? "" : Number(value)) : value,
    }));
  };

  const handleSubmit = (e: React.SubmitEvent) => {
    e.preventDefault();
    console.log("Valeurs du formulaire: ", formData);
  };

  const handleSelectChange = (name: string, value: string) => {
    setFormData((prev) => ({
      ...prev,
      [name]: value,
    }));
  };

  return (
    <DialogPortal>
      <DialogOverlay className="fixed inset-0 bg-black/50 z-50" />
      <DialogContent className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-background p-6 rounded-lg shadow-lg z-50 w-full max-w-lg">
        <DialogTitle>Création d'une nouvelle map</DialogTitle>
        <Form onSubmit={handleSubmit}>
          <div className="space-y-3">
            <FormField>
              <FormLabel className="ml-1">Nom de la map</FormLabel>
              <FormInput
                type="text"
                name="name"
                placeholder="Nom"
                value={formData.name}
                onChange={handleChange}
              />
            </FormField>
            <FormField>
              <FormLabel className="ml-1">Archétype de la map</FormLabel>
              {/* Faire une sélection déroulante parmi les archétypes existant */}
              <Select
                value={formData.archetype}
                onValueChange={(value) =>
                  handleSelectChange("archetype", value)
                }
              >
                <SelectTrigger>
                  <SelectValue placeholder="Sélectionner un archétype" />
                </SelectTrigger>
                <SelectContent>
                  {ARCHETYPES.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {item.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FormField>
            <FormField>
              <FormLabel className="ml-1">Terrain par défaut</FormLabel>
              {/* Faire une sélection déroulante parmi les terrains existant */}
              <Select
                value={formData.defaultTerrain}
                onValueChange={(value) =>
                  handleSelectChange("defaultTerrain", value)
                }
              >
                <SelectTrigger>
                  <SelectValue placeholder="Sélectionner un terrain" />
                </SelectTrigger>
                <SelectContent>
                  {DEFAULT_TERRAINS.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {item.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </FormField>
            <div className="grid grid-cols-2 gap-4">
              <FormField>
                <FormLabel className="ml-1">Largeur</FormLabel>
                <FormInput
                  type="number"
                  placeholder="Largeur"
                  defaultValue={10}
                />
              </FormField>
              <FormField>
                <FormLabel className="ml-1">Hauteur</FormLabel>
                <FormInput
                  type="number"
                  placeholder="Hauteur"
                  defaultValue={10}
                />
              </FormField>
            </div>
            <div className="pt-2 border-t border-border">
              <Button
                variant={"ghost"}
                onClick={() => setShowAdvanced(!showAdvanced)}
              >
                <span>Paramètres avancés</span>
                <CircleChevronDown
                  className={`w-4 h-4 transition-transform duration-200 ${showAdvanced ? "rotate-180" : ""}`}
                />
              </Button>
              {showAdvanced && (
                <div className="grid grid-cols-2 space-y-4 pt-3 pb-1 animate-fadeIn gap-4">
                  <FormField>
                    <FormLabel className="ml-1">
                      Taux de spawn des visiteurs
                    </FormLabel>
                    <FormInput type="number" defaultValue={100} />
                  </FormField>
                  <FormField>
                    <FormLabel className="ml-1">
                      Nombre maximal de visiteurs
                    </FormLabel>
                    <FormInput type="number" defaultValue={100} />
                  </FormField>
                </div>
              )}
            </div>
          </div>
          <DialogFooter>
            <Button type="submit">Créer</Button>
          </DialogFooter>
        </Form>
      </DialogContent>
    </DialogPortal>
  );
}
