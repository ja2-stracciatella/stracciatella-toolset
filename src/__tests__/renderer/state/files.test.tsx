import { it, describe, expect } from 'vitest';
import { createAppStore } from '../../../renderer/state/store';
import {
  changeEditMode,
  changeJson,
  changeSaveMode,
  changeText,
  loadJSON,
  persistJSON,
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

async function setupTestFile(
  readOutput: Partial<InvokableOutput<JsonReadInvokable>> = {},
) {
  const appStore = createAppStore();
  const invokeMock = getInvokeMock();

  resolveJsonRead(invokeMock, readOutput);
  await appStore.dispatch(loadJSON(TEST_FILE));

  return { appStore, invokeMock };
}

describe('files state', () => {
  describe('loadJson', () => {
    it('should load a file without mod values', async () => {
      const { appStore } = await setupTestFile();

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
      const { appStore } = await setupTestFile({
        patch: TEST_PATCH,
      });

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
      const { appStore } = await setupTestFile({
        value: TEST_REPLACE,
      });

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
      const { appStore } = await setupTestFile();

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
              const { appStore } = await setupTestFile(readOutput);

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
              const { appStore } = await setupTestFile(readOutput);

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
                const { appStore } = await setupTestFile(readOutput);

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
              const { appStore } = await setupTestFile(readOutput);

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

  describe('changeSaveMode', () => {
    describe('in visual edit mode', () => {
      async function setup(
        readOutput: Partial<InvokableOutput<JsonReadInvokable>>,
      ) {
        const { appStore, invokeMock } = await setupTestFile(readOutput);

        expect(appStore.getState().files.open[TEST_FILE]?.saveMode).toBe(
          'patch',
        );

        return { appStore, invokeMock };
      }

      [
        {
          name: 'without mod values',
          readOutput: {},
          expectedValue: TEST_VANILLA,
        },
        {
          name: 'with patch mod value',
          readOutput: {
            patch: TEST_PATCH,
          },
          expectedValue: TEST_PATCHED,
        },
      ].forEach(({ name, readOutput, expectedValue }) => {
        describe(name, () => {
          it('should change modified state', async () => {
            const { appStore } = await setup(readOutput);

            appStore.dispatch(
              changeSaveMode({
                filename: TEST_FILE,
                saveMode: 'replace',
              }),
            );

            expect(appStore.getState().files.open[TEST_FILE]).toEqual({
              editMode: 'visual',
              saveMode: 'replace',
              modified: true,
              value: expectedValue,
            });
          });
        });
      });
    });

    describe('in text edit mode', () => {
      describe('patch to replace', () => {
        async function setup(
          readOutput: Partial<InvokableOutput<JsonReadInvokable>>,
        ) {
          const { appStore, invokeMock } = await setupTestFile(readOutput);

          appStore.dispatch(
            changeEditMode({
              filename: TEST_FILE,
              editMode: 'text',
            }),
          );
          expect(appStore.getState().files.open[TEST_FILE]?.saveMode).toBe(
            'patch',
          );
          expect(appStore.getState().files.open[TEST_FILE]?.editMode).toBe(
            'text',
          );

          return {
            appStore,
            invokeMock,
          };
        }

        [
          { name: 'without mod data', readOutput: {}, expected: TEST_VANILLA },
          {
            name: 'with mod data',
            readOutput: {
              patch: TEST_PATCH,
            },
            expected: TEST_PATCHED,
          },
        ].forEach(({ name, readOutput, expected }) => {
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
              it(`should do nothing if value is not ${name}`, async () => {
                const { appStore } = await setup(readOutput);

                appStore.dispatch(
                  changeText({
                    filename: TEST_FILE,
                    value,
                  }),
                );
                appStore.dispatch(
                  changeSaveMode({
                    filename: TEST_FILE,
                    saveMode: 'replace',
                  }),
                );

                expect(appStore.getState().files.open[TEST_FILE]).toEqual({
                  editMode: 'text',
                  saveMode: 'patch',
                  modified: true,
                  value,
                });
              });
            });

            it(`should change editor value to full value`, async () => {
              const { appStore } = await setup(readOutput);

              appStore.dispatch(
                changeSaveMode({
                  filename: TEST_FILE,
                  saveMode: 'replace',
                }),
              );

              expect(appStore.getState().files.open[TEST_FILE]).toEqual({
                editMode: 'text',
                saveMode: 'replace',
                modified: true,
                value: JSON.stringify(expected, null, 4),
              });
            });

            it(`should change editor value to full value when modified`, async () => {
              const { appStore } = await setup(readOutput);
              appStore.dispatch(
                changeText({
                  filename: TEST_FILE,
                  value: JSON.stringify([
                    {
                      ...TEST_PATCH[0],
                      value: 'other patch',
                    },
                  ]),
                }),
              );

              appStore.dispatch(
                changeSaveMode({
                  filename: TEST_FILE,
                  saveMode: 'replace',
                }),
              );

              expect(appStore.getState().files.open[TEST_FILE]).toEqual({
                editMode: 'text',
                saveMode: 'replace',
                modified: true,
                value: JSON.stringify({ test: 'other patch' }, null, 4),
              });
            });
          });
        });
      });

      describe('replace to patch', () => {
        async function setup(
          readOutput: Partial<InvokableOutput<JsonReadInvokable>>,
        ) {
          const { appStore, invokeMock } = await setupTestFile(readOutput);

          appStore.dispatch(
            changeSaveMode({
              filename: TEST_FILE,
              saveMode: 'replace',
            }),
          );
          appStore.dispatch(
            changeEditMode({
              filename: TEST_FILE,
              editMode: 'text',
            }),
          );
          expect(appStore.getState().files.open[TEST_FILE]?.saveMode).toBe(
            'replace',
          );
          expect(appStore.getState().files.open[TEST_FILE]?.editMode).toBe(
            'text',
          );

          return {
            appStore,
            invokeMock,
          };
        }

        [
          {
            name: 'without mod data',
            readOutput: {},
            expectedModified: false,
            expected: [],
          },
          {
            name: 'with mod data',
            readOutput: {
              value: TEST_REPLACE,
            },
            expectedModified: true,
            expected: [
              {
                op: 'replace',
                path: '/test',
                value: 'replace value',
              },
            ],
          },
        ].forEach(({ name, readOutput, expectedModified, expected }) => {
          describe(name, () => {
            [
              {
                name: 'json',
                value: 'foobar',
              },
              {
                name: 'json root',
                value: '"foobar"',
              },
            ].forEach(({ name, value }) => {
              it(`should do nothing if value is not ${name}`, async () => {
                const { appStore } = await setup(readOutput);

                appStore.dispatch(
                  changeText({
                    filename: TEST_FILE,
                    value,
                  }),
                );
                appStore.dispatch(
                  changeSaveMode({
                    filename: TEST_FILE,
                    saveMode: 'patch',
                  }),
                );

                expect(appStore.getState().files.open[TEST_FILE]).toEqual({
                  editMode: 'text',
                  saveMode: 'replace',
                  modified: true,
                  value,
                });
              });
            });

            it(`should change editor value to patch value`, async () => {
              const { appStore } = await setup(readOutput);

              appStore.dispatch(
                changeSaveMode({
                  filename: TEST_FILE,
                  saveMode: 'patch',
                }),
              );

              expect(appStore.getState().files.open[TEST_FILE]).toEqual({
                editMode: 'text',
                saveMode: 'patch',
                modified: expectedModified,
                value: JSON.stringify(expected, null, 4),
              });
            });

            it(`should change editor value to patch value after modification`, async () => {
              const { appStore } = await setup(readOutput);

              appStore.dispatch(
                changeText({
                  filename: TEST_FILE,
                  value: '{}',
                }),
              );
              appStore.dispatch(
                changeSaveMode({
                  filename: TEST_FILE,
                  saveMode: 'patch',
                }),
              );

              expect(appStore.getState().files.open[TEST_FILE]).toEqual({
                editMode: 'text',
                saveMode: 'patch',
                modified: true,
                value: JSON.stringify(
                  [{ op: 'remove', path: '/test' }],
                  null,
                  4,
                ),
              });
            });
          });
        });
      });
    });
  });

  describe('persistJSON', () => {
    describe('in visual edit mode', () => {
      describe('in patch save mode', async () => {
        async function setup(
          readOutput: Partial<InvokableOutput<JsonReadInvokable>>,
        ) {
          const { appStore, invokeMock } = await setupTestFile(readOutput);

          appStore.dispatch(
            changeSaveMode({
              filename: TEST_FILE,
              saveMode: 'patch',
            }),
          );
          expect(appStore.getState().files.open[TEST_FILE]?.saveMode).toBe(
            'patch',
          );
          expect(appStore.getState().files.open[TEST_FILE]?.editMode).toBe(
            'visual',
          );

          return {
            appStore,
            invokeMock,
          };
        }

        [
          {
            name: 'without mod data',
            readOutput: {},
            expectedNotModified: { patch: null, applied: TEST_VANILLA },
          },
          {
            name: 'with mod patch data',
            readOutput: { patch: TEST_PATCH },
            expectedNotModified: {
              patch: TEST_PATCH,
              applied: TEST_PATCHED,
            },
          },
          {
            name: 'with mod replace data',
            readOutput: { value: TEST_REPLACE },
            expectedNotModified: {
              patch: [
                {
                  op: 'replace' as const,
                  path: '/test',
                  value: TEST_REPLACE.test,
                },
              ],
              applied: TEST_REPLACE,
            },
          },
        ].forEach(({ name, readOutput, expectedNotModified }) => {
          describe(name, () => {
            it('should save the correct patch', async () => {
              const { appStore, invokeMock } = await setup(readOutput);

              invokeMock.resolve(
                'json/persist',
                {
                  file: TEST_FILE,
                  value: null,
                  patch: expectedNotModified.patch,
                },
                {
                  schema: TEST_SCHEMA,
                  vanilla: TEST_VANILLA,
                  value: null,
                  patch: expectedNotModified.patch,
                },
              );
              await appStore.dispatch(persistJSON(TEST_FILE));

              expect(
                appStore.getState().files.disk[TEST_FILE]?.persistingError,
              ).toBeNull();
              expect(appStore.getState().files.disk[TEST_FILE]?.data).toEqual({
                schema: TEST_SCHEMA,
                itemSchema: null,
                title: TEST_FILE,
                description: null,
                saveMode: 'patch',
                vanilla: TEST_VANILLA,
                mod: null,
                ...expectedNotModified,
              });
            });

            it('should save the correct patch after modifications', async () => {
              const { appStore, invokeMock } = await setup(readOutput);
              const expectedPatch = [{ op: 'remove' as const, path: '/test' }];

              invokeMock.resolve(
                'json/persist',
                {
                  file: TEST_FILE,
                  value: null,
                  patch: expectedPatch,
                },
                {
                  schema: TEST_SCHEMA,
                  vanilla: TEST_VANILLA,
                  value: null,
                  patch: expectedPatch,
                },
              );
              appStore.dispatch(
                changeJson({
                  filename: TEST_FILE,
                  value: {},
                }),
              );
              await appStore.dispatch(persistJSON(TEST_FILE));

              expect(
                appStore.getState().files.disk[TEST_FILE]?.persistingError,
              ).toBeNull();
              expect(appStore.getState().files.disk[TEST_FILE]?.data).toEqual({
                schema: TEST_SCHEMA,
                itemSchema: null,
                title: TEST_FILE,
                description: null,
                saveMode: 'patch',
                vanilla: TEST_VANILLA,
                mod: null,
                patch: expectedPatch,
                applied: {},
              });
            });
          });
        });
      });

      describe('in replace save mode', () => {
        async function setup(
          readOutput: Partial<InvokableOutput<JsonReadInvokable>>,
        ) {
          const { appStore, invokeMock } = await setupTestFile(readOutput);

          appStore.dispatch(
            changeSaveMode({
              filename: TEST_FILE,
              saveMode: 'replace',
            }),
          );
          expect(appStore.getState().files.open[TEST_FILE]?.saveMode).toBe(
            'replace',
          );
          expect(appStore.getState().files.open[TEST_FILE]?.editMode).toBe(
            'visual',
          );

          return {
            appStore,
            invokeMock,
          };
        }

        [
          {
            name: 'without mod data',
            readOutput: {},
            expectedNotModified: { mod: TEST_VANILLA, applied: TEST_VANILLA },
          },
          {
            name: 'with mod patch data',
            readOutput: { patch: TEST_PATCH },
            expectedNotModified: {
              mod: TEST_PATCHED,
              applied: TEST_PATCHED,
            },
          },
          {
            name: 'with mod replace data',
            readOutput: { value: TEST_REPLACE },
            expectedNotModified: {
              mod: TEST_REPLACE,
              applied: TEST_REPLACE,
            },
          },
        ].forEach(({ name, readOutput, expectedNotModified }) => {
          describe(name, () => {
            it('should save the correct patch', async () => {
              const { appStore, invokeMock } = await setup(readOutput);

              invokeMock.resolve(
                'json/persist',
                {
                  file: TEST_FILE,
                  value: expectedNotModified.mod,
                  patch: null,
                },
                {
                  schema: TEST_SCHEMA,
                  vanilla: TEST_VANILLA,
                  value: expectedNotModified.mod,
                  patch: null,
                },
              );
              await appStore.dispatch(persistJSON(TEST_FILE));

              expect(
                appStore.getState().files.disk[TEST_FILE]?.persistingError,
              ).toBeNull();
              expect(appStore.getState().files.disk[TEST_FILE]?.data).toEqual({
                schema: TEST_SCHEMA,
                itemSchema: null,
                title: TEST_FILE,
                description: null,
                saveMode: 'replace',
                vanilla: TEST_VANILLA,
                patch: null,
                ...expectedNotModified,
              });
            });

            it('should save the correct patch after modifications', async () => {
              const { appStore, invokeMock } = await setup(readOutput);

              invokeMock.resolve(
                'json/persist',
                {
                  file: TEST_FILE,
                  value: {},
                  patch: null,
                },
                {
                  schema: TEST_SCHEMA,
                  vanilla: TEST_VANILLA,
                  value: {},
                  patch: null,
                },
              );
              appStore.dispatch(
                changeJson({
                  filename: TEST_FILE,
                  value: {},
                }),
              );
              await appStore.dispatch(persistJSON(TEST_FILE));

              expect(
                appStore.getState().files.disk[TEST_FILE]?.persistingError,
              ).toBeNull();
              expect(appStore.getState().files.disk[TEST_FILE]?.data).toEqual({
                schema: TEST_SCHEMA,
                itemSchema: null,
                title: TEST_FILE,
                description: null,
                saveMode: 'replace',
                vanilla: TEST_VANILLA,
                mod: {},
                patch: null,
                applied: {},
              });
            });
          });
        });
      });
    });

    describe('in text edit mode', () => {
      describe('in patch mode', () => {
        async function setup(
          readOutput: Partial<InvokableOutput<JsonReadInvokable>>,
        ) {
          const { appStore, invokeMock } = await setupTestFile(readOutput);

          appStore.dispatch(
            changeEditMode({
              filename: TEST_FILE,
              editMode: 'text',
            }),
          );
          expect(appStore.getState().files.open[TEST_FILE]?.saveMode).toBe(
            'patch',
          );
          expect(appStore.getState().files.open[TEST_FILE]?.editMode).toBe(
            'text',
          );

          return {
            appStore,
            invokeMock,
          };
        }

        [
          {
            name: 'valid json',
            value: 'test',
            expected: 'current editor value is not valid JSON',
          },
          {
            name: 'a valid json patch',
            value: '{}',
            expected: 'current editor value is not a valid JSON patch',
          },
        ].forEach(({ name, value, expected }) => {
          it(`should throw an error when the editor value is not ${name}`, async () => {
            const { appStore } = await setup({});
            appStore.dispatch(
              changeText({
                filename: TEST_FILE,
                value,
              }),
            );
            await appStore.dispatch(persistJSON(TEST_FILE));

            expect(
              appStore.getState().files.disk[TEST_FILE]?.persistingError
                ?.message,
            ).toContain(expected);
            expect(appStore.getState().files.open[TEST_FILE]?.modified).toBe(
              true,
            );
          });
        });

        it('should persist the current editor value', async () => {
          const { appStore, invokeMock } = await setup({});
          const patch = [
            {
              op: 'add' as const,
              path: '/test',
              value: 'test',
            },
          ];

          invokeMock.resolve(
            'json/persist',
            {
              file: TEST_FILE,
              value: null,
              patch,
            },
            {
              vanilla: TEST_VANILLA,
              schema: TEST_SCHEMA,
              value: null,
              patch,
            },
          );

          appStore.dispatch(
            changeText({
              filename: TEST_FILE,
              value: JSON.stringify(patch),
            }),
          );
          await appStore.dispatch(persistJSON(TEST_FILE));

          expect(
            appStore.getState().files.disk[TEST_FILE]?.persistingError,
          ).toBeNull();
          expect(appStore.getState().files.disk[TEST_FILE]?.data).toEqual({
            title: TEST_FILE,
            description: null,
            vanilla: TEST_VANILLA,
            schema: TEST_SCHEMA,
            itemSchema: null,
            saveMode: 'patch',
            mod: null,
            patch,
            applied: {
              test: 'test',
            },
          });
          expect(appStore.getState().files.open[TEST_FILE]).toEqual({
            editMode: 'text',
            saveMode: 'patch',
            modified: false,
            value: JSON.stringify(patch),
          });
        });
      });
    });

    describe('in replace mode', () => {
      async function setup(
        readOutput: Partial<InvokableOutput<JsonReadInvokable>>,
      ) {
        const { appStore, invokeMock } = await setupTestFile(readOutput);

        appStore.dispatch(
          changeSaveMode({
            filename: TEST_FILE,
            saveMode: 'replace',
          }),
        );
        appStore.dispatch(
          changeEditMode({
            filename: TEST_FILE,
            editMode: 'text',
          }),
        );
        expect(appStore.getState().files.open[TEST_FILE]?.saveMode).toBe(
          'replace',
        );
        expect(appStore.getState().files.open[TEST_FILE]?.editMode).toBe(
          'text',
        );

        return {
          appStore,
          invokeMock,
        };
      }

      [
        {
          name: 'valid json',
          value: 'test',
          expected: 'current editor value is not valid JSON',
        },
        {
          name: 'a valid json root',
          value: '1',
          expected: 'current editor value is not a valid JSON root',
        },
      ].forEach(({ name, value, expected }) => {
        it(`should throw an error when the editor value is not ${name}`, async () => {
          const { appStore } = await setup({});
          appStore.dispatch(
            changeText({
              filename: TEST_FILE,
              value,
            }),
          );
          await appStore.dispatch(persistJSON(TEST_FILE));

          expect(
            appStore.getState().files.disk[TEST_FILE]?.persistingError?.message,
          ).toContain(expected);
          expect(appStore.getState().files.open[TEST_FILE]?.modified).toBe(
            true,
          );
        });
      });

      it('should persist the current editor value', async () => {
        const { appStore, invokeMock } = await setup({});
        const value = {
          test: 'test',
        };

        invokeMock.resolve(
          'json/persist',
          {
            file: TEST_FILE,
            value,
            patch: null,
          },
          {
            vanilla: TEST_VANILLA,
            schema: TEST_SCHEMA,
            value,
            patch: null,
          },
        );

        appStore.dispatch(
          changeText({
            filename: TEST_FILE,
            value: JSON.stringify(value),
          }),
        );
        await appStore.dispatch(persistJSON(TEST_FILE));

        expect(
          appStore.getState().files.disk[TEST_FILE]?.persistingError,
        ).toBeNull();
        expect(appStore.getState().files.disk[TEST_FILE]?.data).toEqual({
          title: TEST_FILE,
          description: null,
          vanilla: TEST_VANILLA,
          schema: TEST_SCHEMA,
          itemSchema: null,
          saveMode: 'replace',
          mod: value,
          patch: null,
          applied: {
            test: 'test',
          },
        });
        expect(appStore.getState().files.open[TEST_FILE]).toEqual({
          editMode: 'text',
          saveMode: 'replace',
          modified: false,
          value: JSON.stringify(value),
        });
      });
    });
  });
});
