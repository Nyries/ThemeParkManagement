/*
  Warnings:

  - You are about to drop the column `playerId` on the `Park` table. All the data in the column will be lost.
  - A unique constraint covering the columns `[email]` on the table `Player` will be added. If there are existing duplicate values, this will fail.
  - Added the required column `characterId` to the `Park` table without a default value. This is not possible if the table is not empty.
  - Added the required column `email` to the `Player` table without a default value. This is not possible if the table is not empty.

*/
-- DropForeignKey
ALTER TABLE "Park" DROP CONSTRAINT "Park_playerId_fkey";

-- DropIndex
DROP INDEX "Park_playerId_key";

-- AlterTable
ALTER TABLE "Park" DROP COLUMN "playerId",
ADD COLUMN     "characterId" TEXT NOT NULL;

-- AlterTable
ALTER TABLE "Player" ADD COLUMN     "email" TEXT NOT NULL;

-- CreateTable
CREATE TABLE "Character" (
    "id" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    "role" TEXT NOT NULL,
    "level" INTEGER NOT NULL DEFAULT 1,
    "playerId" TEXT NOT NULL,

    CONSTRAINT "Character_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "Player_email_key" ON "Player"("email");

-- AddForeignKey
ALTER TABLE "Character" ADD CONSTRAINT "Character_playerId_fkey" FOREIGN KEY ("playerId") REFERENCES "Player"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "Park" ADD CONSTRAINT "Park_characterId_fkey" FOREIGN KEY ("characterId") REFERENCES "Character"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
