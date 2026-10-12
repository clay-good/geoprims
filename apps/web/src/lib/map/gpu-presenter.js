// Present the canonical Canvas2D map frame through luma.gl. Keeping GPU setup
// here gives the map one small backend boundary while the same scene remains
// available for export, interaction, and the no-GPU fallback.
async function loadLuma() {
  const [{ luma }, { BackgroundTextureModel }, { webgl2Adapter }, { webgpuAdapter }] = await Promise.all([
    import('@luma.gl/core'),
    import('@luma.gl/engine'),
    import('@luma.gl/webgl'),
    import('@luma.gl/webgpu'),
  ]);
  return { luma, BackgroundTextureModel, webgl2Adapter, webgpuAdapter };
}

export async function createGpuPresenter(output, source, load = loadLuma) {
  const { luma, BackgroundTextureModel, webgl2Adapter, webgpuAdapter } = await load();
  const canvasProps = {
    canvas: output,
    autoResize: false,
    useDevicePixels: false,
    alphaMode: 'opaque',
  };
  let device;
  try {
    device = await luma.createDevice({
      type: 'webgpu',
      adapters: [webgpuAdapter],
      waitForPageLoad: false,
      createCanvasContext: canvasProps,
    });
  } catch {
    device = await luma.createDevice({
      type: 'webgl',
      adapters: [webgl2Adapter],
      waitForPageLoad: false,
      createCanvasContext: canvasProps,
    });
  }
  const context = device.getDefaultCanvasContext();
  let texture = null;
  let model = null;

  function resize() {
    if (texture?.width === source.width && texture?.height === source.height) return;
    texture?.destroy();
    texture = device.createTexture({
      id: 'geoprims-map-frame',
      width: source.width,
      height: source.height,
      format: 'rgba8unorm',
      sampler: { minFilter: 'nearest', magFilter: 'nearest' },
    });
    context.setDrawingBufferSize(source.width, source.height);
    if (model) model.setProps({ backgroundTexture: texture });
    else model = new BackgroundTextureModel(device, { backgroundTexture: texture, flipY: true });
  }

  return {
    backend: device.info.type,
    present() {
      resize();
      texture.copyExternalImage({ image: source });
      model.predraw(device.commandEncoder);
      const pass = device.beginRenderPass({ clearColor: [0, 0, 0, 1] });
      model.draw(pass);
      pass.end();
      device.submit();
    },
    destroy() {
      model?.destroy();
      texture?.destroy();
      device.destroy();
    },
  };
}
