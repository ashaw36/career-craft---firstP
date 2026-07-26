import { invoke } from "@tauri-apps/api/core";

export function mountE2EBridge(){
 const host=document.createElement("section");host.id="e2e-ipc-bridge";host.setAttribute("aria-label","E2E IPC bridge");
 host.innerHTML='<textarea id="e2e-ipc-request" aria-label="E2E IPC request"></textarea><button id="e2e-ipc-send">Invoke</button><pre id="e2e-ipc-result" aria-live="polite"></pre>';
 document.body.append(host);
 const input=host.querySelector<HTMLTextAreaElement>("#e2e-ipc-request")!,result=host.querySelector<HTMLElement>("#e2e-ipc-result")!;
 host.querySelector("button")!.addEventListener("click",async()=>{result.textContent="pending";try{const request=JSON.parse(input.value)as{command:string;payload?:unknown};const args=request.command==="parse_jd"?request.payload:{payload:request.payload};result.textContent=JSON.stringify(await invoke(request.command,args as Record<string,unknown>))}catch(error){result.textContent=JSON.stringify({success:false,error:{code:"TRANSPORT",message:String(error)}})}});
}
