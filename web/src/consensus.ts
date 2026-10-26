export interface Candidate { value: number; foreground: number; repairedBytes: number; version: number }

/** Consecutive distinct captures; uncertainty and gaps break the sequence. */
export class Consensus {
  private candidate: Candidate | null = null;
  private count = 0;
  private started = 0;
  private lastTime = -Infinity;
  private lastFrame = -1;
  reset() { this.candidate=null;this.count=0;this.started=0;this.lastTime=-Infinity;this.lastFrame=-1; }
  push(result: Candidate|null, time: number, frame: number): { confirmed: Candidate|null; count: number } {
    if(frame<=this.lastFrame || time<=this.lastTime || !Number.isFinite(time)) return {confirmed:null,count:0};
    const gap=time-this.lastTime;
    this.lastTime=time;this.lastFrame=frame;
    if(!result) {this.count=0;this.candidate=null;return {confirmed:null,count:0};}
    if(!this.candidate || this.candidate.value!==result.value || this.candidate.foreground!==result.foreground
      || this.candidate.version!==result.version || gap>700 || time-this.started>1500) {
      this.candidate=result;this.count=1;this.started=time;
    } else {this.count++;}
    return {confirmed:this.count>=3?result:null,count:this.count};
  }
}
