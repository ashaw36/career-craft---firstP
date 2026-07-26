import { describe, expect, it, vi } from "vitest";
import { completeOnboarding, needsOnboarding, renderOnboarding } from "../../src/features/home/onboarding";
describe("first-run onboarding", () => {
  it("is shown until completion is persisted", () => {
    const storage = { getItem: vi.fn().mockReturnValue(null), setItem: vi.fn() };
    expect(needsOnboarding(storage)).toBe(true);
    completeOnboarding(storage);
    expect(storage.setItem).toHaveBeenCalledWith("careercraft:onboarding-complete", "true");
    expect(renderOnboarding()).toContain("数据默认保存在本机");
  });
});
