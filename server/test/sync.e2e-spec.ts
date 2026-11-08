import { Test, TestingModule } from '@nestjs/testing';
import { INestApplication } from '@nestjs/common';
import request from 'supertest';
import { App } from 'supertest/types';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { AppModule } from './../src/app.module';

// Exercises the real multipart upload path (FileFieldsInterceptor, backed by
// multer) over HTTP, so a multer upgrade is proven end to end rather than
// only through the controller unit tests, which pass files in directly.
describe('Sync upload over HTTP (e2e)', () => {
  let app: INestApplication<App>;
  let dataDir: string;
  let token: string;

  beforeEach(async () => {
    dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sync-e2e-'));
    process.env.DATA_DIR = dataDir;

    const moduleFixture: TestingModule = await Test.createTestingModule({
      imports: [AppModule],
    }).compile();

    app = moduleFixture.createNestApplication();
    await app.init();

    const res = await request(app.getHttpServer())
      .post('/auth/register')
      .send({ email: 'e2e@example.test', password: `pw-${Date.now()}` })
      .expect(201);
    token = (res.body as { access_token: string }).access_token;
  });

  afterEach(async () => {
    await app.close();
    fs.rmSync(dataDir, { recursive: true, force: true });
    delete process.env.DATA_DIR;
  });

  it('accepts a multipart upload and returns the same bytes on download', async () => {
    await request(app.getHttpServer())
      .put('/sync')
      .set('Authorization', `Bearer ${token}`)
      .field('device_name', 'e2e-rig')
      .attach('db', Buffer.from('db-bytes'), 'database.db')
      .attach('db_wal', Buffer.from('wal-bytes'), 'database.db-wal')
      .expect(200)
      .expect({ status: 'ok' });

    const res = await request(app.getHttpServer())
      .get('/sync')
      .set('Authorization', `Bearer ${token}`)
      .expect(200);
    const body = res.body as {
      db: string;
      db_shm: string | null;
      db_wal: string | null;
      device_name: string;
    };

    expect(Buffer.from(body.db, 'base64').toString()).toBe('db-bytes');
    expect(Buffer.from(body.db_wal ?? '', 'base64').toString()).toBe(
      'wal-bytes',
    );
    expect(body.db_shm).toBeNull();
    expect(body.device_name).toBe('e2e-rig');
  });

  it('rejects more files in a field than its maxCount allows', async () => {
    await request(app.getHttpServer())
      .put('/sync')
      .set('Authorization', `Bearer ${token}`)
      .attach('db', Buffer.from('one'), 'a.db')
      .attach('db', Buffer.from('two'), 'b.db')
      .expect(400);
  });

  it('rejects an upload without a token', async () => {
    await request(app.getHttpServer())
      .put('/sync')
      .attach('db', Buffer.from('db-bytes'), 'database.db')
      .expect(401);
  });
});
