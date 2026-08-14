/*
  Warnings:

  - You are about to drop the column `characterId` on the `Park` table. All the data in the column will be lost.
  - You are about to drop the column `balance` on the `Player` table. All the data in the column will be lost.
  - You are about to drop the `Character` table. If the table is not empty, all the data it contains will be lost.
  - Added the required column `companyId` to the `Park` table without a default value. This is not possible if the table is not empty.

*/
-- DropForeignKey
ALTER TABLE "Character" DROP CONSTRAINT "Character_playerId_fkey";

-- DropForeignKey
ALTER TABLE "Park" DROP CONSTRAINT "Park_characterId_fkey";

-- AlterTable
ALTER TABLE "Park" DROP COLUMN "characterId",
ADD COLUMN     "balance" DOUBLE PRECISION NOT NULL DEFAULT 1000.0,
ADD COLUMN     "companyId" TEXT NOT NULL;

-- AlterTable
ALTER TABLE "Player" DROP COLUMN "balance",
ADD COLUMN     "premiumBalance" DOUBLE PRECISION NOT NULL DEFAULT 0;

-- DropTable
DROP TABLE "Character";

-- CreateTable
CREATE TABLE "Company" (
    "id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "role" TEXT NOT NULL,
    "level" INTEGER NOT NULL DEFAULT 1,
    "balance" DOUBLE PRECISION NOT NULL DEFAULT 0,
    "playerId" TEXT NOT NULL,

    CONSTRAINT "Company_pkey" PRIMARY KEY ("id")
);

-- AddForeignKey
ALTER TABLE "Company" ADD CONSTRAINT "Company_playerId_fkey" FOREIGN KEY ("playerId") REFERENCES "Player"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "Park" ADD CONSTRAINT "Park_companyId_fkey" FOREIGN KEY ("companyId") REFERENCES "Company"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
