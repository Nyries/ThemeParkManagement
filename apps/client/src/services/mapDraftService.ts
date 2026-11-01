import { fetchApi } from "./api";

export const mapService = {
  saveDraft: async (mapData: any, userId?: string) => {
    return fetchApi("/map-draft/new", {
      method: "POST",
      body: JSON.stringify({ userId, mapData }),
    });
  },
};
