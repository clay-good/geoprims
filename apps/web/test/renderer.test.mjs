import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createGpuPresenter } from '../src/lib/map/gpu-presenter.js';

function harness(type) {
  const calls = [];
  const context = { setDrawingBufferSize: (...args) => calls.push(['size', ...args]) };
  const texture = {
    width: 640,
    height: 320,
    copyExternalImage: (options) => calls.push(['copy', options.image]),
    destroy: () => calls.push(['texture-destroy']),
  };
  const pass = { end: () => calls.push(['end']) };
  const device = {
    info: { type },
    commandEncoder: {},
    getDefaultCanvasContext: () => context,
    createTexture: (props) => {
      calls.push(['texture', props.width, props.height]);
      return texture;
    },
    beginRenderPass: () => (calls.push(['pass']), pass),
    submit: () => calls.push(['submit']),
    destroy: () => calls.push(['device-destroy']),
  };
  class BackgroundTextureModel {
    constructor(_device, props) { calls.push(['model', props.backgroundTexture, props.flipY]); }
    predraw() { calls.push(['predraw']); }
    draw() { calls.push(['draw']); }
    setProps() { calls.push(['model-update']); }
    destroy() { calls.push(['model-destroy']); }
  }
  const load = async () => ({
    luma: {
      createDevice: async (props) => {
        calls.push(['device', props.type]);
        if (props.type === 'webgpu' && type === 'webgl') throw new Error('WebGPU unavailable');
        return device;
      },
    },
    BackgroundTextureModel,
    webgpuAdapter: 'webgpu',
    webgl2Adapter: 'webgl2',
  });
  return { calls, load };
}

for (const backend of ['webgpu', 'webgl']) {
  test(`${backend} presents the identical canonical map frame`, async () => {
    const source = { width: 640, height: 320 };
    const { calls, load } = harness(backend);
    const presenter = await createGpuPresenter({}, source, load);
    assert.equal(presenter.backend, backend);
    presenter.present();
    assert.deepEqual(calls.filter((call) => call[0] === 'device'), backend === 'webgpu'
      ? [['device', 'webgpu']]
      : [['device', 'webgpu'], ['device', 'webgl']]);
    assert.ok(calls.some((call) => call[0] === 'copy' && call[1] === source));
    assert.deepEqual(calls.slice(-5).map((call) => call[0]), ['predraw', 'pass', 'draw', 'end', 'submit']);
    assert.ok(calls.findIndex((call) => call[0] === 'copy') < calls.findIndex((call) => call[0] === 'draw'));
    assert.equal(calls.at(-1)[0], 'submit');
  });
}
