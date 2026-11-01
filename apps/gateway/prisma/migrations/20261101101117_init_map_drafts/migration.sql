-- AlterTable
ALTER TABLE "Park" ALTER COLUMN "balance" SET DEFAULT 10000.0;

-- CreateTable
CREATE TABLE "map_drafts" (
    "id" TEXT NOT NULL,
    "userId" TEXT,
    "mapData" JSONB NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "map_drafts_pkey" PRIMARY KEY ("id")
);
