require "yaml"

require "trove"
require "woollib/common"

require "./Queued"

module Wool
  class Users
    mserializable

    getter chest : Trove::Chest

    def add(u : User) : Id
      raise Exception.new "User #{u.to_json} already exists" if @chest.has_key! u.id, "type"
      @chest.set u.id, "", Wool.to_tj u
      u.id
    end

    def get(ui : Id) : User?
      Wool.from_tj User, (@chest.get ui).not_nil! rescue nil
    end

    def delete(ui : Id) : Id
      @chest.transaction do |tx|
        tx.delete ui
        tx.where({"pseudonym.user_id" => ui.string}) { |ii| tx.delete ii }
      end
      ui
    end

    def add(p : Pseudonym) : Id
      raise Exception.new "Pseudonym #{p.to_json} already exists" if @chest.has_key! p.id, "type"
      @chest.set p.id, "", Wool.to_tj p
      p.id
    end

    def get(s : Site, name : String) : User?
      ii = @chest.where({"pseudonym.site" => (String.from_json s.to_json),
                         "pseudonym.name" => name}).first rescue return nil
      i = Wool.from_tj Pseudonym, (@chest.get ii).not_nil!
      Wool.from_tj User, (@chest.get i.user_id).not_nil! rescue nil
    end

    def delete(i : Pseudonym) : Id
      @chest.delete i.id
      i.id
    end

    def push(ui : Id, c : Command(Users) | Command(Sweater)) : Queued
      q = Queued.new ui, c
      @chest.set q.id, "", Wool.to_tj q
      q
    end

    def pull(limit : UInt64 = UInt64::MAX, from : Id? = nil, &block : Queued ->) : Nil
      @chest.where({"type" => "queued"}) { |i| yield Wool.from_tj Queued, (@chest.get i).not_nil! }
    end

    def pull(limit : UInt64 = UInt64::MAX, from : Id? = nil) : Array(Queued)
      r = Array(Queued).new
      pull(limit, from) { |q| r << q }
      r
    end
  end
end
