const KEY = "careercraft:onboarding-complete";
export function needsOnboarding(storage: Pick<Storage, "getItem"> = localStorage): boolean { return storage.getItem(KEY) !== "true"; }
export function completeOnboarding(storage: Pick<Storage, "setItem"> = localStorage): void { storage.setItem(KEY, "true"); }
export function renderOnboarding(): string {
  return `<div class="modal-backdrop"><section class="onboarding" role="dialog" aria-modal="true" aria-labelledby="onboarding-title"><p class="eyebrow">本地优先的职业工作台</p><h1 id="onboarding-title">先整理一段真实经历</h1><p>CareerCraft 会以你的原始经历为依据，帮助你创建角色档案、简历和岗位匹配。数据默认保存在本机。</p><ol><li>添加或导入一段经历</li><li>确认 AI 整理后的内容</li><li>创建目标角色并生成简历</li></ol><div class="dialog-actions"><button class="secondary" data-onboarding-skip>稍后再说</button><button class="primary" data-onboarding-start>开始整理</button></div></section></div>`;
}
