import {
  describe,
  it,
  expect,
  vi,
  beforeAll,
  afterAll,
  beforeEach,
} from "vitest";
import express from "express";
import type { Server as HttpServer } from "http";
import type { AddressInfo } from "net";
import { Prisma, type PrismaClient } from "../generated/prisma/client";
import { registerRoutes } from "../routes";

const findUniqueMock = vi.fn();
const parkUpdateMock = vi.fn();
const companyUpdateMock = vi.fn();

const mockPrisma = {
  park: { findUnique: findUniqueMock, update: parkUpdateMock },
  company: { update: companyUpdateMock },
} as unknown as PrismaClient;

function notFoundError() {
  return new Prisma.PrismaClientKnownRequestError("Record not found", {
    code: "P2025",
    clientVersion: "test",
  });
}

describe("balance routes (integration)", () => {
  let httpServer: HttpServer;
  let baseUrl: string;

  beforeAll(async () => {
    const app = express();
    app.use(express.json());
    registerRoutes(app, mockPrisma);

    httpServer = app.listen(0);
    await new Promise<void>((resolve) =>
      httpServer.once("listening", resolve),
    );
    const port = (httpServer.address() as AddressInfo).port;
    baseUrl = `http://localhost:${port}`;
  });

  afterAll(() => {
    httpServer.close();
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("GET /park/:parkId/balance", () => {
    it("returns the park and company balance when the park exists", async () => {
      findUniqueMock.mockResolvedValue({
        balance: 1000,
        company: { balance: 0 },
      });

      const res = await fetch(`${baseUrl}/park/park-1/balance`);

      expect(res.status).toBe(200);
      expect(await res.json()).toEqual({
        parkBalance: 1000,
        companyBalance: 0,
      });
      expect(findUniqueMock).toHaveBeenCalledWith({
        where: { id: "park-1" },
        select: { balance: true, company: { select: { balance: true } } },
      });
    });

    it("returns 404 when the park does not exist", async () => {
      findUniqueMock.mockResolvedValue(null);

      const res = await fetch(`${baseUrl}/park/does-not-exist/balance`);

      expect(res.status).toBe(404);
    });

    it("returns 500 when the database call fails unexpectedly", async () => {
      findUniqueMock.mockRejectedValue(new Error("connection lost"));

      const res = await fetch(`${baseUrl}/park/park-1/balance`);

      expect(res.status).toBe(500);
    });
  });

  describe("PUT /park/:parkId/balance", () => {
    it("updates and returns the new park balance", async () => {
      parkUpdateMock.mockResolvedValue({ balance: 2500 });

      const res = await fetch(`${baseUrl}/park/park-1/balance`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ balance: 2500 }),
      });

      expect(res.status).toBe(200);
      expect(await res.json()).toEqual({ parkBalance: 2500 });
      expect(parkUpdateMock).toHaveBeenCalledWith({
        where: { id: "park-1" },
        data: { balance: 2500 },
      });
    });

    it("returns 400 when balance is not a number", async () => {
      const res = await fetch(`${baseUrl}/park/park-1/balance`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ balance: "not-a-number" }),
      });

      expect(res.status).toBe(400);
      expect(parkUpdateMock).not.toHaveBeenCalled();
    });

    it("returns 404 when the park does not exist", async () => {
      parkUpdateMock.mockRejectedValue(notFoundError());

      const res = await fetch(`${baseUrl}/park/does-not-exist/balance`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ balance: 100 }),
      });

      expect(res.status).toBe(404);
    });

    it("returns 500 on an unexpected database error", async () => {
      parkUpdateMock.mockRejectedValue(new Error("connection lost"));

      const res = await fetch(`${baseUrl}/park/park-1/balance`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ balance: 100 }),
      });

      expect(res.status).toBe(500);
    });
  });

  describe("PUT /company/:companyId/balance", () => {
    it("updates and returns the new company balance", async () => {
      companyUpdateMock.mockResolvedValue({ balance: 300 });

      const res = await fetch(`${baseUrl}/company/company-1/balance`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ balance: 300 }),
      });

      expect(res.status).toBe(200);
      expect(await res.json()).toEqual({ companyBalance: 300 });
      expect(companyUpdateMock).toHaveBeenCalledWith({
        where: { id: "company-1" },
        data: { balance: 300 },
      });
    });

    it("returns 400 when balance is not a number", async () => {
      const res = await fetch(`${baseUrl}/company/company-1/balance`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ balance: null }),
      });

      expect(res.status).toBe(400);
      expect(companyUpdateMock).not.toHaveBeenCalled();
    });

    it("returns 404 when the company does not exist", async () => {
      companyUpdateMock.mockRejectedValue(notFoundError());

      const res = await fetch(`${baseUrl}/company/does-not-exist/balance`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ balance: 100 }),
      });

      expect(res.status).toBe(404);
    });

    it("returns 500 on an unexpected database error", async () => {
      companyUpdateMock.mockRejectedValue(new Error("connection lost"));

      const res = await fetch(`${baseUrl}/company/company-1/balance`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ balance: 100 }),
      });

      expect(res.status).toBe(500);
    });
  });
});
