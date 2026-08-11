export const targets = [
  { package: "gwt-darwin-arm64", target: "bun-darwin-arm64" },
  { package: "gwt-darwin-x64", target: "bun-darwin-x64-baseline" },
  { package: "gwt-linux-arm64-gnu", target: "bun-linux-arm64" },
  { package: "gwt-linux-arm64-musl", target: "bun-linux-arm64-musl" },
  { package: "gwt-linux-x64-gnu", target: "bun-linux-x64-baseline" },
  { package: "gwt-linux-x64-musl", target: "bun-linux-x64-musl-baseline" },
] as const;
