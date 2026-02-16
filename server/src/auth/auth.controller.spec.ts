import { Test, TestingModule } from '@nestjs/testing';
import { JwtModule } from '@nestjs/jwt';
import { AuthController } from './auth.controller';
import { AuthService } from './auth.service';
import { UnauthorizedException, ConflictException } from '@nestjs/common';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

describe('AuthController', () => {
  let controller: AuthController;
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'auth-test-'));
    process.env.DATA_DIR = tmpDir;

    const module: TestingModule = await Test.createTestingModule({
      imports: [
        JwtModule.register({
          secret: 'test-secret',
          signOptions: { expiresIn: '1h' },
        }),
      ],
      controllers: [AuthController],
      providers: [AuthService],
    }).compile();

    controller = module.get<AuthController>(AuthController);
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    delete process.env.DATA_DIR;
  });

  it('POST /auth/register - creates account and returns JWT', async () => {
    const result = await controller.register({
      email: 'test@example.com',
      password: 'password123',
    });

    expect(result.access_token).toBeDefined();
    expect(typeof result.access_token).toBe('string');
    expect(result.access_token.split('.')).toHaveLength(3); // JWT format
  });

  it('POST /auth/register - rejects duplicate email', async () => {
    await controller.register({
      email: 'dupe@example.com',
      password: 'password123',
    });

    await expect(
      controller.register({
        email: 'dupe@example.com',
        password: 'different',
      }),
    ).rejects.toThrow(ConflictException);
  });

  it('POST /auth/login - authenticates and returns JWT', async () => {
    await controller.register({
      email: 'login@example.com',
      password: 'mypassword',
    });

    const result = await controller.login({
      email: 'login@example.com',
      password: 'mypassword',
    });

    expect(result.access_token).toBeDefined();
    expect(result.access_token.split('.')).toHaveLength(3);
  });

  it('POST /auth/login - rejects wrong password', async () => {
    await controller.register({
      email: 'wrong@example.com',
      password: 'correct',
    });

    await expect(
      controller.login({
        email: 'wrong@example.com',
        password: 'incorrect',
      }),
    ).rejects.toThrow(UnauthorizedException);
  });

  it('POST /auth/login - rejects unknown email', async () => {
    await expect(
      controller.login({
        email: 'nobody@example.com',
        password: 'anything',
      }),
    ).rejects.toThrow(UnauthorizedException);
  });

  it('register returns different tokens for different users', async () => {
    const r1 = await controller.register({
      email: 'user1@example.com',
      password: 'pass1',
    });
    const r2 = await controller.register({
      email: 'user2@example.com',
      password: 'pass2',
    });

    expect(r1.access_token).not.toBe(r2.access_token);
  });
});
