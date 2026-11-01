import { Router } from "express";
import { Prisma, PrismaClient } from "../generated/prisma/client";

export function createCompanyRouter(prisma: PrismaClient) {
  const router = Router();

  router.put("/:companyId/balance", async (req, res) => {
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
      if (
        err instanceof Prisma.PrismaClientKnownRequestError &&
        err.code === "P2025"
      ) {
        return res.status(404).json({ error: "Company introuvable" });
      }
      res.status(500).json({ error: "Erreur de connexion BDD", details: err });
    }
  });

  return router;
}
