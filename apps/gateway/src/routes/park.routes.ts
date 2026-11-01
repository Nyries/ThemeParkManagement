import { Router } from "express";
import { Prisma, type PrismaClient } from "../generated/prisma/client";

export function createParkRouter(prisma: PrismaClient) {
  const router = Router();
  router.get("/:parkId/balance", async (req, res) => {
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

  router.put("/:parkId/balance", async (req, res) => {
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
      if (
        err instanceof Prisma.PrismaClientKnownRequestError &&
        err.code === "P2025"
      ) {
        return res.status(404).json({ error: "Park not found" });
      }
      return res
        .status(500)
        .json({ error: "Database connection failed", details: err });
    }
  });
  return router;
}
