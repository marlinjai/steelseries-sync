import { Test, TestingModule } from '@nestjs/testing';
import { SyncController } from './sync.controller';
import { SyncService } from './sync.service';
import { NotFoundException } from '@nestjs/common';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

describe('SyncController', () => {
  let controller: SyncController;
  let service: SyncService;
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sync-test-'));
    process.env.DATA_DIR = tmpDir;

    const module: TestingModule = await Test.createTestingModule({
      controllers: [SyncController],
      providers: [SyncService],
    }).compile();

    controller = module.get<SyncController>(SyncController);
    service = module.get<SyncService>(SyncService);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    delete process.env.DATA_DIR;
  });

  const mockReq = { user: { sub: 'test-user-123' } };

  function createMockFile(
    content: string,
    fieldname: string,
  ): Express.Multer.File {
    return {
      fieldname,
      originalname: fieldname,
      encoding: '7bit',
      mimetype: 'application/octet-stream',
      buffer: Buffer.from(content),
      size: content.length,
      stream: null as any,
      destination: '',
      filename: '',
      path: '',
    };
  }

  it('PUT /sync - uploads config files', async () => {
    const files = {
      db: [createMockFile('db-content', 'db')],
      db_shm: [createMockFile('shm-content', 'db_shm')],
      db_wal: [createMockFile('wal-content', 'db_wal')],
    };

    const result = await controller.upload(files, 'gaming-pc', mockReq);

    expect(result).toEqual({ status: 'ok' });

    // Verify files were written to disk
    const userDir = path.join(tmpDir, 'users', 'test-user-123');
    expect(fs.existsSync(path.join(userDir, 'database.db'))).toBe(true);
    expect(fs.readFileSync(path.join(userDir, 'database.db'), 'utf-8')).toBe(
      'db-content',
    );
    expect(
      fs.readFileSync(path.join(userDir, 'database.db-shm'), 'utf-8'),
    ).toBe('shm-content');
    expect(
      fs.readFileSync(path.join(userDir, 'database.db-wal'), 'utf-8'),
    ).toBe('wal-content');
  });

  it('GET /sync - downloads config files as base64', async () => {
    // First upload
    const files = {
      db: [createMockFile('db-content', 'db')],
      db_shm: [createMockFile('shm-content', 'db_shm')],
      db_wal: [createMockFile('wal-content', 'db_wal')],
    };
    await controller.upload(files, 'gaming-pc', mockReq);

    // Then download
    const result = await controller.download(mockReq);

    expect(result.db).toBe(Buffer.from('db-content').toString('base64'));
    expect(result.db_shm).toBe(Buffer.from('shm-content').toString('base64'));
    expect(result.db_wal).toBe(Buffer.from('wal-content').toString('base64'));
    expect(result.device_name).toBe('gaming-pc');
    expect(result.last_modified).toBeDefined();
  });

  it('GET /sync/meta - returns metadata only', async () => {
    // First upload
    const files = {
      db: [createMockFile('db-content', 'db')],
    };
    await controller.upload(files, 'work-laptop', mockReq);

    // Then get meta
    const result = await controller.meta(mockReq);

    expect(result.device_name).toBe('work-laptop');
    expect(result.last_modified).toBeDefined();
    // Should not contain file data
    expect((result as any).db).toBeUndefined();
  });

  it('GET /sync - returns 404 when no config exists', async () => {
    await expect(controller.download(mockReq)).rejects.toThrow(
      NotFoundException,
    );
  });

  it('GET /sync/meta - returns 404 when no config exists', async () => {
    await expect(controller.meta(mockReq)).rejects.toThrow(NotFoundException);
  });

  it('PUT /sync - handles upload with only db file (no shm/wal)', async () => {
    const files = {
      db: [createMockFile('db-only', 'db')],
    };

    const result = await controller.upload(files, 'minimal-pc', mockReq);
    expect(result).toEqual({ status: 'ok' });

    const downloaded = await controller.download(mockReq);
    expect(downloaded.db).toBe(Buffer.from('db-only').toString('base64'));
    expect(downloaded.db_shm).toBeNull();
    expect(downloaded.db_wal).toBeNull();
  });
});
