const RELAY_TIERS: { maxVisitors: number; everyNthTick: number }[] = [
  { maxVisitors: 100, everyNthTick: 1 }, // full regim (~20Hz)
  { maxVisitors: 1000, everyNthTick: 2 },
  { maxVisitors: 5000, everyNthTick: 5 },
  { maxVisitors: Infinity, everyNthTick: 10 },
];

export function everyNthTickFor(visitorCount: number): number {
  const tier = RELAY_TIERS.find((t) => visitorCount <= t.maxVisitors);
  return tier
    ? tier.everyNthTick
    : RELAY_TIERS[RELAY_TIERS.length - 1].everyNthTick;
}

export function shouldRelay(tickCount: number, visitorCount: number): boolean {
  return tickCount % everyNthTickFor(visitorCount) === 0;
}
