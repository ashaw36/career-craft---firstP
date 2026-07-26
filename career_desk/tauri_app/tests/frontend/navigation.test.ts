import { describe, expect, it } from "vitest";
import { NavigationStore, routeFromHash, routes } from "../../src/shared/state/navigation";

describe("navigation", () => {
  it("contains exactly the eight frozen product pages", () => expect(routes.map((r) => r.label)).toEqual(["首页", "经历库", "角色档案", "简历", "岗位匹配", "技能图谱", "学习路径", "设置"]));
  it("falls back safely for unknown routes", () => expect(routeFromHash("#/missing")).toBe("home"));
  it("notifies when navigating", () => {
    const store = new NavigationStore("#/home");
    let changed = false;
    store.addEventListener("change", () => { changed = true; });
    store.navigate("jobs");
    expect(store.current).toBe("jobs"); expect(changed).toBe(true);
  });
});
