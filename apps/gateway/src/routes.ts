import type { Express } from "express";
import { Prisma, PrismaClient } from "./generated/prisma/client";

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

  app.get("/park/:parkId/balance", async (req, res) => {
    try {
      const park = await prisma.park.findUnique({
        where: { id: req.params.parkId },
        select: { balance: true, company: { select: { balance: true } } },
      });
      if (!park) {
        return res.status(404).json({ error: "Park not found" });
      }
      res.status(200).json({
        parkBalance: park.balance,
        companyBalance: park.company.balance,
      });
    } catch (error) {
      res
        .status(500)
        .json({ error: "Database connection failed", details: error });
    }
  });

  app.put("/park/:parkId/balance", async (req, res) => {
    try {
      const { balance } = req.body;
      if (typeof balance !== "number") {
        return res.status(400).json({ error: "balance must be a number" });
      }
      const park = await prisma.park.update({
        where: { id: req.params.parkId },
        data: { balance },
      });
      res.status(200).json({ parkBalance: park.balance });
    } catch (err) {
      if (err instanceof Prisma.PrismaClientKnownRequestError && err.code === "P2025") {
        return res.status(404).json({ error: "Park not found" });
      }
      return res
        .status(500)
        .json({ error: "Database connection failed", details: err });
    }
  });

  app.put("/company/:companyId/balance", async (req, res) => {
    try {
      const { balance } = req.body;
      if (typeof balance !== "number") {
        return res.status(400).json({ error: "balance doit être un nombre" });
      }
      const company = await prisma.company.update({
        where: { id: req.params.companyId },
        data: { balance },
      });
      res.status(200).json({ companyBalance: company.balance });
    } catch (err) {
      if (err instanceof Prisma.PrismaClientKnownRequestError && err.code === "P2025") {
        return res.status(404).json({ error: "Company introuvable" });
      }
      res
        .status(500)
        .json({ error: "Erreur de connexion BDD", details: err });
    }
  });
}
