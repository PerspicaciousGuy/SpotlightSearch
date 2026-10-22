// Keep IPC bounded when typing outruns the backend: one running request and one
// replaceable pending request. There is no debounce timer on the fast path.
export class LatestSearchQueue<Request, Response> {
  private revision = 0;
  private pending?: { request: Request; revision: number };
  private running = false;

  constructor(
    private readonly perform: (request: Request) => Promise<Response>,
    private readonly receive: (response: Response, request: Request) => void,
    private readonly fail: (error: unknown) => void,
    private readonly settled: () => void,
  ) {}

  submit(request: Request) {
    this.pending = { request, revision: ++this.revision };
    if (!this.running) void this.drain();
  }

  // Background refreshes must not discard a running search or replace a newer
  // pending one, so they only start when nothing else is queued.
  submitIfIdle(request: Request) {
    if (!this.running && !this.pending) this.submit(request);
  }

  clear() {
    ++this.revision;
    this.pending = undefined;
  }

  private async drain() {
    this.running = true;
    while (this.pending) {
      const current = this.pending;
      this.pending = undefined;
      try {
        const response = await this.perform(current.request);
        if (current.revision === this.revision) this.receive(response, current.request);
      } catch (error) {
        if (current.revision === this.revision) this.fail(error);
      } finally {
        if (current.revision === this.revision) this.settled();
      }
    }
    this.running = false;
  }
}
