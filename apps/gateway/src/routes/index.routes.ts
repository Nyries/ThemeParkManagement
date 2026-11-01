import type { Express } from "express";
import type { PrismaClient } from "../generated/prisma/client";
import { createParkRouter } from "./park.routes";
import { createCompanyRouter } from "./company.routes";
import { createMapDraftsRouter } from "./map-drafts.routes";

export function registerRoutes(app: Express, prisma: PrismaClient) {
  app.get("/health", async (req, res) => {
    try {
      const count = await prisma.player.count();
      res.status(200).json({ status: "Gateway active", playerCount: count });
    } catch (error) {
      res
        .status(500)
        .json({ error: "Erreur de connexion BDD", details: error });
    }
  });

  app.use("/park", createParkRouter(prisma));
  app.use("/company", createCompanyRouter(prisma));
  app.use("/map-drafts", createMapDraftsRouter(prisma));
}