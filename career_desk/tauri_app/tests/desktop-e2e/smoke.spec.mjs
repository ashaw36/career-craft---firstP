import assert from "node:assert/strict";
import { invoke, ok } from "./helpers.mjs";
describe("embedded IPC smoke",()=>{it("opens a native session and invokes health",async()=>{assert.equal(ok(await invoke("health")).status,"ok")})});
