import {
  Injectable,
  UnauthorizedException,
  ConflictException,
} from '@nestjs/common';
import { JwtService } from '@nestjs/jwt';
import * as bcrypt from 'bcrypt';
import * as fs from 'fs';
import * as path from 'path';

interface User {
  id: string;
  email: string;
  passwordHash: string;
}

interface UserStore {
  users: User[];
}

@Injectable()
export class AuthService {
  private readonly storePath: string;

  constructor(private readonly jwtService: JwtService) {
    const dataDir = process.env.DATA_DIR || path.join(process.cwd(), 'data');
    fs.mkdirSync(dataDir, { recursive: true });
    this.storePath = path.join(dataDir, 'users.json');
  }

  private readStore(): UserStore {
    if (!fs.existsSync(this.storePath)) {
      return { users: [] };
    }
    return JSON.parse(fs.readFileSync(this.storePath, 'utf-8')) as UserStore;
  }

  private writeStore(store: UserStore): void {
    fs.writeFileSync(this.storePath, JSON.stringify(store, null, 2));
  }

  async register(
    email: string,
    password: string,
  ): Promise<{ access_token: string }> {
    const store = this.readStore();

    if (store.users.find((u) => u.email === email)) {
      throw new ConflictException('Email already registered');
    }

    const id = this.generateId();
    const passwordHash = await bcrypt.hash(password, 10);
    store.users.push({ id, email, passwordHash });
    this.writeStore(store);

    const token = this.jwtService.sign({ sub: id, email });
    return { access_token: token };
  }

  async login(
    email: string,
    password: string,
  ): Promise<{ access_token: string }> {
    const store = this.readStore();
    const user = store.users.find((u) => u.email === email);

    if (!user) {
      throw new UnauthorizedException('Invalid credentials');
    }

    const valid = await bcrypt.compare(password, user.passwordHash);
    if (!valid) {
      throw new UnauthorizedException('Invalid credentials');
    }

    const token = this.jwtService.sign({ sub: user.id, email: user.email });
    return { access_token: token };
  }

  private generateId(): string {
    return (
      Date.now().toString(36) + Math.random().toString(36).substring(2, 10)
    );
  }
}
