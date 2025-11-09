import z from 'zod';
import { InvokableDefinition } from '.';

type Category = 'image';

const RENDER_INPUT_SCHEMA = z.object({
  file: z.string(),
  subimage: z.nullable(z.number()),
});

const RENDER_OUTPUT_SCHEMA = z.string();

export type ImageRenderInvokable = InvokableDefinition<
  Category,
  'render',
  z.infer<typeof RENDER_INPUT_SCHEMA>,
  z.infer<typeof RENDER_OUTPUT_SCHEMA>
>;

export const imageRenderInvokableDefinition: ImageRenderInvokable = {
  name: 'image/render',
  inputSchema: RENDER_INPUT_SCHEMA,
  outputSchema: RENDER_OUTPUT_SCHEMA,
};

const READ_METADATA_INPUT_SCHEMA = z.object({
  file: z.string(),
});

const SUBIMAGE_SCHEMA = z.object({
  width: z.number(),
  height: z.number(),
  offset_x: z.number(),
  offset_y: z.number(),
});

const IMAGE_METADATA_SCHEMA = z.object({
  images: z.array(SUBIMAGE_SCHEMA),
});

export type ImageMetadata = z.infer<typeof IMAGE_METADATA_SCHEMA>;

export type ImageReadMetadataInvokable = InvokableDefinition<
  Category,
  'readMetadata',
  z.infer<typeof READ_METADATA_INPUT_SCHEMA>,
  z.infer<typeof IMAGE_METADATA_SCHEMA>
>;

export const imageReadMetadataInvokableDefinition: ImageReadMetadataInvokable =
  {
    name: 'image/readMetadata',
    inputSchema: READ_METADATA_INPUT_SCHEMA,
    outputSchema: IMAGE_METADATA_SCHEMA,
  };
