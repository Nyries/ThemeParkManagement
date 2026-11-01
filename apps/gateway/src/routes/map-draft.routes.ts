import { Router } from "express";
import { PrismaClient } from "../generated/prisma/client";

export function createMapDraftRouter(prisma: PrismaClient) {
  const router = Router();

  router.post("/new", async (req, res) => {
    try {
      const { userId, mapData } = req.body;

      if (!mapData) {
        return res.status(400).json({ error: "mapData is required" });
      }

      const draft = await prisma.mapDraft.create({
        data: {
          userId: userId || null,
          mapData: mapData,
        },
      });

      return res.status(201).json({
        message: "Draft saved successfully",
        draftId: draft.id,
        updatedAt: draft.updatedAt,
      });
    } catch (error) {
      console.error("Error saving map draft: ", error);
      return res
        .status(500)
        .json({ error: "Internal server error", details: error });
    }
  });
  
  return router;
}
