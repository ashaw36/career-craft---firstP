export const routes = [
  { id: "home", label: "首页", icon: "⌂" }, { id: "experiences", label: "经历库", icon: "▤" },
  { id: "personas", label: "角色档案", icon: "◎" }, { id: "resumes", label: "简历", icon: "▧" },
  { id: "jobs", label: "岗位匹配", icon: "◇" }, { id: "skills", label: "技能图谱", icon: "⌘" },
  { id: "learning", label: "学习路径", icon: "↗" }, { id: "settings", label: "设置", icon: "⚙" }
] as const;
export type RouteId = (typeof routes)[number]["id"];
export function routeFromHash(hash: string): RouteId {
  const candidate = hash.replace(/^#\/?/, "");
  return routes.some((route) => route.id === candidate) ? candidate as RouteId : "home";
}
export class NavigationStore extends EventTarget {
  current: RouteId;
  constructor(hash = window.location.hash) { super(); this.current = routeFromHash(hash); }
  navigate(route: RouteId): void {
    if (route === this.current) return;
    this.current = route;
    window.location.hash = `/${route}`;
    this.dispatchEvent(new Event("change"));
  }
}
