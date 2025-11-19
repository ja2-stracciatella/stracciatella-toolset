import { it, describe, expect } from 'vitest';
import { createAppStore } from '../../../renderer/state/store';
import {
  changeEditMode,
  changeJson,
  changeText,
  loadJSON,
} from '../../../renderer/state/files';
import { getInvokeMock, InvokeMock } from '../test-utils/invoke';
import { miniSerializeError } from '@reduxjs/toolkit';
import { JsonReadInvokable } from 'src/common/invokables/jsons';
import { InvokableOutput } from 'src/common/invokables';

const TEST_FILE = 'testfile.json';
const TEST_SCHEMA = {
  type: 'object',
  properties: { test: { type: 'string' } },
  required: ['test'],
};
const TEST_VANILLA = { test: 'vanilla value' };
const TEST_REPLACE = { test: 'replace value' };
const TEST_PATCH = [
  { op: 'replace' as const, path: '/test', value: 'patch value' },
];
const TEST_PATCHED = { test: 'patch value' };
const TEST_DEFAULT_DISK_STATE = {
  loading: false,
  loadingError: null,
  persisting: false,
  persistingError: null,
  data: {
    title: TEST_FILE,
    description: null,
    schema: TEST_SCHEMA,
    itemSchema: null,
    vanilla: TEST_VANILLA,
    mod: null,
    patch: null,
    applied: TEST_VANILLA,
    saveMode: 'patch',
  },
};

function resolveJsonRead(
  invokeMock: InvokeMock,
  data: Partial<InvokableOutput<JsonReadInvokable>> = {},
) {
  invokeMock.resolve(
    'json/read',
    { file: TEST_FILE },
    {
      schema: TEST_SCHEMA,
      vanilla: TEST_VANILLA,
      value: null,
      patch: null,
      ...data,
    },
  );
}

describe('files state', () => {
  describe('loadJson', () => {
    it('should load a file without mod values', async () => {
      const appStore = createAppStore();
      const invokeMock = getInvokeMock();

      resolveJsonRead(invokeMock);

      await appStore.dispatch(loadJSON(TEST_FILE));

      expect(appStore.getState().files.disk[TEST_FILE]).toEqual(
        TEST_DEFAULT_DISK_STATE,
      );
      expect(appStore.getState().files.open[TEST_FILE]).toEqual({
        saveMode: 'patch',
        editMode: 'visual',
        modified: false,
        value: TEST_VANILLA,
      });
    });

    it('should load a file with a mod patch', async () => {
      const appStore = createAppStore();
      const invokeMock = getInvokeMock();

      resolveJsonRead(invokeMock, {
        patch: TEST_PATCH,
      });

      await appStore.dispatch(loadJSON(TEST_FILE));

      expect(appStore.getState().files.disk[TEST_FILE]).toEqual({
        ...TEST_DEFAULT_DISK_STATE,
        data: {
          ...TEST_DEFAULT_DISK_STATE.data,
          patch: TEST_PATCH,
          applied: TEST_PATCHED,
        },
      });
      expect(appStore.getState().files.open[TEST_FILE]).toEqual({
        saveMode: 'patch',
        editMode: 'visual',
        modified: false,
        value: TEST_PATCHED,
      });
    });

    it('should load a file with a mod replacement value', async () => {
      const appStore = createAppStore();
      const invokeMock = getInvokeMock();

      resolveJsonRead(invokeMock, {
        value: TEST_REPLACE,
      });

      await appStore.dispatch(loadJSON(TEST_FILE));

      expect(appStore.getState().files.disk[TEST_FILE]).toEqual({
        ...TEST_DEFAULT_DISK_STATE,
        data: {
          ...TEST_DEFAULT_DISK_STATE.data,
          saveMode: 'replace',
          mod: TEST_REPLACE,
          applied: TEST_REPLACE,
        },
      });
      expect(appStore.getState().files.open[TEST_FILE]).toEqual({
        saveMode: 'replace',
        editMode: 'visual',
        modified: false,
        value: TEST_REPLACE,
      });
    });

    it('should set an error on invoke failures', async () => {
      const appStore = createAppStore();
      const invokeMock = getInvokeMock();
      const error = new Error('test error');

      invokeMock.reject('json/read', { file: TEST_FILE }, error);

      await appStore.dispatch(loadJSON(TEST_FILE));

      expect(appStore.getState().files.disk[TEST_FILE]).toEqual({
        loading: false,
        loadingError: miniSerializeError(error),
        persisting: false,
        persistingError: null,
        data: null,
      });
    });
  });

  describe('changeJson', () => {
    it('should update a json file and set the modified flag', async () => {
      const appStore = createAppStore();
      const invokeMock = getInvokeMock();

      resolveJsonRead(invokeMock);

      await appStore.dispatch(loadJSON(TEST_FILE));
      appStore.dispatch(
        changeJson({
          filename: TEST_FILE,
          value: {
            test: 'foo without bar',
          },
        }),
      );

      expect(appStore.getState().files.disk[TEST_FILE]).toEqual(
        TEST_DEFAULT_DISK_STATE,
      );
      expect(appStore.getState().files.open[TEST_FILE]).toEqual({
        saveMode: 'patch',
        editMode: 'visual',
        modified: true,
        value: { test: 'foo without bar' },
      });
    });
  });

  describe('changeEditMode', () => {
    describe('visual to text', () => {
      [
        {
          name: 'without preexisting data in patch save mode',
          readOutput: {},
          expectedNotModified: {
            saveMode: 'patch',
            editMode: 'text',
            modified: false,
            value: JSON.stringify([], null, 4),
          },
          expectedModified: {
            saveMode: 'patch',
            editMode: 'text',
            modified: true,
            value: JSON.stringify(
              [{ op: 'replace', path: '/test', value: 'foo without bar' }],
              null,
              4,
            ),
          },
        },
        {
          name: 'with preexisting data in patch save mode',
          readOutput: {
            patch: TEST_PATCH,
          },
          expectedNotModified: {
            saveMode: 'patch',
            editMode: 'text',
            modified: false,
            value: JSON.stringify(TEST_PATCH, null, 4),
          },
          expectedModified: {
            saveMode: 'patch',
            editMode: 'text',
            modified: true,
            value: JSON.stringify(
              [{ op: 'replace', path: '/test', value: 'foo without bar' }],
              null,
              4,
            ),
          },
        },
        {
          name: 'with preexisting data in replace save mode',
          readOutput: {
            value: TEST_REPLACE,
          },
          expectedNotModified: {
            saveMode: 'replace',
            editMode: 'text',
            modified: false,
            value: JSON.stringify(TEST_REPLACE, null, 4),
          },
          expectedModified: {
            saveMode: 'replace',
            editMode: 'text',
            modified: true,
            value: JSON.stringify(
              {
                test: 'foo without bar',
              },
              null,
              4,
            ),
          },
        },
      ].forEach(
        ({ name, readOutput, expectedNotModified, expectedModified }) => {
          describe(name, () => {
            it('should switch edit mode from visual to text when file was not modified', async () => {
              const appStore = createAppStore();
              const invokeMock = getInvokeMock();

              resolveJsonRead(invokeMock, readOutput);
              await appStore.dispatch(loadJSON(TEST_FILE));

              appStore.dispatch(
                changeEditMode({
                  filename: TEST_FILE,
                  editMode: 'text',
                }),
              );

              expect(appStore.getState().files.open[TEST_FILE]).toEqual(
                expectedNotModified,
              );
            });

            it('should switch edit mode when file was modified', async () => {
              const appStore = createAppStore();
              const invokeMock = getInvokeMock();

              resolveJsonRead(invokeMock, readOutput);
              await appStore.dispatch(loadJSON(TEST_FILE));

              appStore.dispatch(
                changeJson({
                  filename: TEST_FILE,
                  value: {
                    test: 'foo without bar',
                  },
                }),
              );
              appStore.dispatch(
                changeEditMode({
                  filename: TEST_FILE,
                  editMode: 'text',
                }),
              );

              expect(appStore.getState().files.open[TEST_FILE]).toEqual(
                expectedModified,
              );
            });
          });
        },
      );
    });

    // Note that these tests depend on the correct behavior of the visual to text
    describe('text to visual', () => {
      [
        {
          name: 'without preexisting data in patch save mode',
          readOutput: {},
          modification: TEST_PATCH,
          expectedSaveMode: 'patch',
          expectedNotModified: {
            editMode: 'visual',
            modified: false,
            saveMode: 'patch',
            value: TEST_VANILLA,
          },
          expectedModified: {
            editMode: 'visual',
            modified: true,
            saveMode: 'patch',
            value: TEST_PATCHED,
          },
        },
        {
          name: 'with preexisting data in patch save mode',
          readOutput: {
            patch: TEST_PATCH,
          },
          modification: [
            {
              ...TEST_PATCH[0],
              value: 'other patch',
            },
          ],
          expectedSaveMode: 'patch',
          expectedNotModified: {
            saveMode: 'patch',
            editMode: 'visual',
            modified: false,
            value: TEST_PATCHED,
          },
          expectedModified: {
            saveMode: 'patch',
            editMode: 'visual',
            modified: true,
            value: {
              test: 'other patch',
            },
          },
        },
        {
          name: 'with preexisting data in replace save mode',
          readOutput: {
            value: TEST_REPLACE,
          },
          modification: {
            test: 'other value',
          },
          expectedSaveMode: 'replace',
          expectedNotModified: {
            saveMode: 'replace',
            editMode: 'visual',
            modified: false,
            value: TEST_REPLACE,
          },
          expectedModified: {
            saveMode: 'replace',
            editMode: 'visual',
            modified: true,
            value: {
              test: 'other value',
            },
          },
        },
      ].forEach(
        ({
          name,
          readOutput,
          modification,
          expectedSaveMode,
          expectedNotModified,
          expectedModified,
        }) => {
          describe(name, () => {
            [
              {
                name: 'json',
                value: 'foobar',
              },
              {
                name: 'patch',
                value: '"foobar"',
              },
            ].forEach(({ name, value }) => {
              it(`should do nothing if the text is not a valid ${name} value`, async () => {
                const appStore = createAppStore();
                const invokeMock = getInvokeMock();

                resolveJsonRead(invokeMock, readOutput);
                await appStore.dispatch(loadJSON(TEST_FILE));
                appStore.dispatch(
                  changeEditMode({
                    filename: TEST_FILE,
                    editMode: 'text',
                  }),
                );
                appStore.dispatch(
                  changeText({
                    filename: TEST_FILE,
                    value,
                  }),
                );
                appStore.dispatch(
                  changeEditMode({
                    filename: TEST_FILE,
                    editMode: 'visual',
                  }),
                );

                expect(appStore.getState().files.open[TEST_FILE]).toEqual({
                  editMode: 'text',
                  modified: true,
                  saveMode: expectedSaveMode,
                  value,
                });
              });
            });

            it('should switch edit mode for non modified files', async () => {
              const appStore = createAppStore();
              const invokeMock = getInvokeMock();

              resolveJsonRead(invokeMock, readOutput);
              await appStore.dispatch(loadJSON(TEST_FILE));
              appStore.dispatch(
                changeEditMode({
                  filename: TEST_FILE,
                  editMode: 'text',
                }),
              );
              appStore.dispatch(
                changeEditMode({
                  filename: TEST_FILE,
                  editMode: 'visual',
                }),
              );

              expect(appStore.getState().files.open[TEST_FILE]).toEqual(
                expectedNotModified,
              );
            });

            it('should switch edit mode for modified files', async () => {
              const appStore = createAppStore();
              const invokeMock = getInvokeMock();

              resolveJsonRead(invokeMock, readOutput);
              await appStore.dispatch(loadJSON(TEST_FILE));
              appStore.dispatch(
                changeEditMode({
                  filename: TEST_FILE,
                  editMode: 'text',
                }),
              );
              appStore.dispatch(
                changeText({
                  filename: TEST_FILE,
                  value: JSON.stringify(modification),
                }),
              );
              appStore.dispatch(
                changeEditMode({
                  filename: TEST_FILE,
                  editMode: 'visual',
                }),
              );

              expect(appStore.getState().files.open[TEST_FILE]).toEqual(
                expectedModified,
              );
            });
          });
        },
      );
    });
  });
});
