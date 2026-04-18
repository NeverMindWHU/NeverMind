import bubbleBgDefault from "../images/光合日历气泡球背景.png";
import spiritAir from "../images/spirit_air.png";
import spiritLight from "../images/spirit_light.png";
import spiritNutrition from "../images/spirit_nutrition.png";
import spiritSoil from "../images/spirit_soil.png";
import spiritWater from "../images/spirit_water.png";

export const GUANGHE_BUBBLE_SKIN_KEY = "guanghe-bubble-skin";

export type BubbleSkinId =
  | "initial"
  | "light"
  | "water"
  | "air"
  | "nutrition"
  | "soil";

export const SKIN_SRC: Record<BubbleSkinId, string> = {
  initial: bubbleBgDefault,
  light: spiritLight,
  water: spiritWater,
  air: spiritAir,
  nutrition: spiritNutrition,
  soil: spiritSoil,
};

/** 右键菜单中的五灵精灵（与 images/spirit_*.png 对应） */
export const SPIRIT_MENU_ITEMS: {
  id: Exclude<BubbleSkinId, "initial">;
  label: string;
}[] = [
  { id: "light", label: "光" },
  { id: "water", label: "水" },
  { id: "air", label: "空气" },
  { id: "nutrition", label: "营养" },
  { id: "soil", label: "土壤" },
];

export function parseStoredSkin(raw: string | null): BubbleSkinId {
  if (
    raw === "initial" ||
    raw === "light" ||
    raw === "water" ||
    raw === "air" ||
    raw === "nutrition" ||
    raw === "soil"
  ) {
    return raw;
  }
  return "initial";
}

export function readBubbleSkin(): BubbleSkinId {
  try {
    return parseStoredSkin(localStorage.getItem(GUANGHE_BUBBLE_SKIN_KEY));
  } catch {
    return "initial";
  }
}

export function writeBubbleSkin(id: BubbleSkinId) {
  try {
    localStorage.setItem(GUANGHE_BUBBLE_SKIN_KEY, id);
  } catch {
    /* ignore */
  }
}
