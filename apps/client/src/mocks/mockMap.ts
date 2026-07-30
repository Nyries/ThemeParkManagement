export type TerrainMaterial = "grass" | "water";
export type InfrastructureKind = "path" | "ramp" | "stairs";

export const WIDTH = 50;
export const HEIGHT = 30;

export function generateMockTerrain(): TerrainMaterial[][] {
  const matrix: TerrainMaterial[][] = Array.from({ length: HEIGHT }, () =>
    Array.from({ length: WIDTH }, () => "grass" as TerrainMaterial),
  );
  matrix[10][20] = "water";
  matrix[10][21] = "water";
  matrix[11][20] = "water";
  matrix[11][21] = "water";
  return matrix;
}

export function generateMockInfrastructure(): (InfrastructureKind | null)[][] {
  const infrastructure: (InfrastructureKind | null)[][] = Array.from(
    { length: HEIGHT },
    () => Array.from({ length: WIDTH }, () => null),
  );

  for (let x = 5; x < 45; x++) {
    infrastructure[15][x] = "path";
  }

  return infrastructure;
}
